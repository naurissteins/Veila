use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

use super::{CheckStatus, DoctorSummary};

const IDLE_SERVICE: &str = "veila-idle.service";
const LOCK_AFTER_KEY: &str = "VEILA_IDLE_LOCK_AFTER";
const SLEEP_FLAG_KEY: &str = "VEILA_IDLE_SLEEP_FLAG";

pub(super) fn check_idle_service(summary: &mut DoctorSummary) {
    check_idle_env(summary);

    let output = Command::new("systemctl")
        .args([
            "--user",
            "show",
            IDLE_SERVICE,
            "--property=LoadState",
            "--property=ActiveState",
            "--property=SubState",
            "--property=UnitFileState",
            "--property=FragmentPath",
            "--no-pager",
        ])
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            println!("idle_service.systemctl=unavailable");
            summary.record(
                "idle_service",
                CheckStatus::Warning,
                format!("failed to run systemctl --user: {error}"),
            );
            return;
        }
    };

    if !output.status.success() {
        println!("idle_service.systemctl=error");
        summary.record(
            "idle_service",
            CheckStatus::Warning,
            command_failure_detail(&output.stderr),
        );
        return;
    }

    println!("idle_service.systemctl=ok");
    let properties = parse_systemctl_show(&String::from_utf8_lossy(&output.stdout));
    let load_state = print_property(&properties, "LoadState", "idle_service.load_state");
    let active_state = print_property(&properties, "ActiveState", "idle_service.active_state");
    print_property(&properties, "SubState", "idle_service.sub_state");
    let unit_file_state =
        print_property(&properties, "UnitFileState", "idle_service.unit_file_state");
    print_property(&properties, "FragmentPath", "idle_service.fragment_path");

    match (load_state.as_deref(), active_state.as_deref()) {
        (Some("loaded"), Some("active")) => summary.record(
            "idle_service",
            CheckStatus::Ok,
            "veila-idle.service is loaded and active",
        ),
        (Some("loaded"), Some(active)) => summary.record(
            "idle_service",
            CheckStatus::Warning,
            format!("veila-idle.service is loaded but active state is {active}"),
        ),
        (Some("not-found"), _) => summary.record(
            "idle_service",
            CheckStatus::Warning,
            "veila-idle.service is not installed in the user service manager",
        ),
        (Some(load), _) => summary.record(
            "idle_service",
            CheckStatus::Warning,
            format!("veila-idle.service load state is {load}"),
        ),
        _ if unit_file_state.as_deref() == Some("enabled") => summary.record(
            "idle_service",
            CheckStatus::Warning,
            "veila-idle.service is enabled but state could not be determined",
        ),
        _ => summary.record(
            "idle_service",
            CheckStatus::Warning,
            "veila-idle.service state could not be determined",
        ),
    }
}

fn check_idle_env(summary: &mut DoctorSummary) {
    let path = idle_env_path();
    println!("idle_service.env_path={}", path.display());

    if !path.exists() {
        println!("idle_service.env=missing");
        println!("idle_service.lock_after=300");
        println!("idle_service.sleep_flag=--lock-before-sleep");
        summary.record(
            "idle_env",
            CheckStatus::Ok,
            "idle service environment file is absent; packaged defaults apply",
        );
        return;
    }

    println!("idle_service.env=present");
    match parse_idle_env_file(&path) {
        Ok(values) => validate_idle_env(summary, &values),
        Err(error) => summary.record(
            "idle_env",
            CheckStatus::Warning,
            format!("failed to read idle service environment file: {error}"),
        ),
    }
}

fn validate_idle_env(summary: &mut DoctorSummary, values: &HashMap<String, String>) {
    let lock_after = values
        .get(LOCK_AFTER_KEY)
        .map(String::as_str)
        .unwrap_or("300");
    let sleep_flag = values
        .get(SLEEP_FLAG_KEY)
        .map(String::as_str)
        .unwrap_or("--lock-before-sleep");

    println!("idle_service.lock_after={lock_after}");
    println!(
        "idle_service.sleep_flag={}",
        if sleep_flag.is_empty() {
            "disabled"
        } else {
            sleep_flag
        }
    );

    if parse_positive_seconds(lock_after).is_none() {
        summary.record(
            "idle_env",
            CheckStatus::Warning,
            format!("{LOCK_AFTER_KEY} must be a positive integer number of seconds"),
        );
        return;
    }

    if !sleep_flag.is_empty() && sleep_flag != "--lock-before-sleep" {
        summary.record(
            "idle_env",
            CheckStatus::Warning,
            format!("{SLEEP_FLAG_KEY} should be empty or --lock-before-sleep"),
        );
        return;
    }

    summary.record(
        "idle_env",
        CheckStatus::Ok,
        "idle service environment values look valid",
    );
}

fn idle_env_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".config/veila/idle.env")
}

fn parse_idle_env_file(path: &Path) -> std::io::Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_env_content(&content))
}

fn parse_env_content(content: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for line in content.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_string(), trim_env_value(value.trim()));
        }
    }
    values
}

fn trim_env_value(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
        .to_string()
}

fn parse_positive_seconds(value: &str) -> Option<u64> {
    value.parse::<u64>().ok().filter(|seconds| *seconds > 0)
}

fn parse_systemctl_show(output: &str) -> HashMap<String, String> {
    let mut properties = HashMap::new();
    for line in output.lines() {
        if let Some((key, value)) = line.split_once('=') {
            properties.insert(key.to_string(), value.to_string());
        }
    }
    properties
}

fn print_property(
    properties: &HashMap<String, String>,
    key: &str,
    output_key: &str,
) -> Option<String> {
    let value = properties
        .get(key)
        .filter(|value| !value.is_empty())
        .cloned();
    println!("{output_key}={}", value.as_deref().unwrap_or("unknown"));
    value
}

fn command_failure_detail(stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr).trim().to_string();
    if detail.is_empty() {
        "systemctl --user show veila-idle.service failed".to_string()
    } else {
        format!("systemctl --user show veila-idle.service failed: {detail}")
    }
}

#[cfg(test)]
mod tests {
    use super::{LOCK_AFTER_KEY, SLEEP_FLAG_KEY, parse_env_content, parse_positive_seconds};

    #[test]
    fn parses_idle_env_values() {
        let values = parse_env_content(
            r#"
            # comment
            VEILA_IDLE_LOCK_AFTER=600
            VEILA_IDLE_SLEEP_FLAG="--lock-before-sleep"
            "#,
        );

        assert_eq!(values.get(LOCK_AFTER_KEY).map(String::as_str), Some("600"));
        assert_eq!(
            values.get(SLEEP_FLAG_KEY).map(String::as_str),
            Some("--lock-before-sleep")
        );
    }

    #[test]
    fn parses_empty_sleep_flag() {
        let values = parse_env_content("VEILA_IDLE_SLEEP_FLAG=\n");

        assert_eq!(values.get(SLEEP_FLAG_KEY).map(String::as_str), Some(""));
    }

    #[test]
    fn rejects_zero_seconds() {
        assert_eq!(parse_positive_seconds("0"), None);
        assert_eq!(parse_positive_seconds("600"), Some(600));
        assert_eq!(parse_positive_seconds("oops"), None);
    }
}
