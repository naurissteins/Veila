use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use super::{
    super::{DAEMON_SERVICE, IDLE_SERVICE},
    CheckStatus, DoctorSummary,
    systemd::{ShowError, UnitState, print_unit_state, show_user_unit},
};

const LEGACY_DAEMON_SERVICE: &str = "veilad.service";
const LOCK_AFTER_KEY: &str = "VEILA_IDLE_LOCK_AFTER";
const SLEEP_FLAG_KEY: &str = "VEILA_IDLE_SLEEP_FLAG";

pub(super) fn check_daemon_service(summary: &mut DoctorSummary) {
    check_legacy_daemon_enablement(summary);

    let Some(state) = show_unit(DAEMON_SERVICE, "daemon_service", summary) else {
        return;
    };

    match (
        state.load.as_deref(),
        state.active.as_deref(),
        state.unit_file.as_deref(),
    ) {
        (Some("loaded"), Some("active"), _) => summary.record(
            "daemon_service",
            CheckStatus::Ok,
            "veila.service is loaded and active",
        ),
        (Some("loaded"), Some(active), Some("enabled")) => summary.record(
            "daemon_service",
            CheckStatus::Warning,
            format!("veila.service is enabled but its active state is {active}"),
        ),
        (Some("loaded"), _, _) => summary.record(
            "daemon_service",
            CheckStatus::Ok,
            "veila.service is installed but not running; fine when your compositor starts `veila daemon`",
        ),
        (Some("not-found"), _, _) => summary.record(
            "daemon_service",
            CheckStatus::Warning,
            "veila.service is not installed in the user service manager",
        ),
        _ => summary.record(
            "daemon_service",
            CheckStatus::Warning,
            "veila.service state could not be determined",
        ),
    }
}

fn check_legacy_daemon_enablement(summary: &mut DoctorSummary) {
    let links = legacy_daemon_links(&user_unit_config_dir());
    if links.is_empty() {
        println!("daemon_service.legacy_enablement=none");
        return;
    }

    for link in &links {
        println!("daemon_service.legacy_enablement={}", link.display());
    }
    summary.record(
        "daemon_service_legacy",
        CheckStatus::Warning,
        format!(
            "{LEGACY_DAEMON_SERVICE} is still enabled under its old name; it keeps working for now, but migrate with `rm {}` and `systemctl --user enable {DAEMON_SERVICE}`",
            links
                .iter()
                .map(|link| link.display().to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ),
    );
}

fn legacy_daemon_links(unit_config_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(unit_config_dir) else {
        return Vec::new();
    };

    let mut links: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "wants")
        })
        .map(|path| path.join(LEGACY_DAEMON_SERVICE))
        .filter(|link| link.symlink_metadata().is_ok())
        .collect();
    links.sort();
    links
}

fn user_unit_config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("systemd/user")
}

pub(super) fn check_idle_service(summary: &mut DoctorSummary) {
    check_idle_env(summary);

    let Some(state) = show_unit(IDLE_SERVICE, "idle_service", summary) else {
        return;
    };

    match (state.load.as_deref(), state.active.as_deref()) {
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
        _ if state.unit_file.as_deref() == Some("enabled") => summary.record(
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

fn show_unit(unit: &str, prefix: &str, summary: &mut DoctorSummary) -> Option<UnitState> {
    match show_user_unit(unit) {
        Ok(properties) => {
            println!("{prefix}.systemctl=ok");
            Some(print_unit_state(&properties, prefix))
        }
        Err(ShowError::Unavailable(detail)) => {
            println!("{prefix}.systemctl=unavailable");
            summary.record(prefix, CheckStatus::Warning, detail);
            None
        }
        Err(ShowError::Failed(detail)) => {
            println!("{prefix}.systemctl=error");
            summary.record(prefix, CheckStatus::Warning, detail);
            None
        }
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

#[cfg(test)]
mod tests {
    use super::{
        LEGACY_DAEMON_SERVICE, LOCK_AFTER_KEY, SLEEP_FLAG_KEY, legacy_daemon_links,
        parse_env_content, parse_positive_seconds,
    };

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

    #[test]
    fn finds_legacy_daemon_enablement_links() {
        let root = std::env::temp_dir().join(format!(
            "veila-doctor-legacy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let wants = root.join("graphical-session.target.wants");
        std::fs::create_dir_all(&wants).expect("wants dir");
        std::fs::create_dir_all(root.join("default.target.wants")).expect("other wants dir");
        std::os::unix::fs::symlink(
            "/usr/lib/systemd/user/veilad.service",
            wants.join(LEGACY_DAEMON_SERVICE),
        )
        .expect("dangling legacy link");
        std::os::unix::fs::symlink(
            "/usr/lib/systemd/user/veila.service",
            wants.join("veila.service"),
        )
        .expect("current link");

        assert_eq!(
            legacy_daemon_links(&root),
            vec![wants.join(LEGACY_DAEMON_SERVICE)]
        );
        assert!(legacy_daemon_links(&root.join("missing")).is_empty());

        std::fs::remove_dir_all(root).ok();
    }
}
