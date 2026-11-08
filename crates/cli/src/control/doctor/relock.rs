use super::{CheckStatus, DoctorSummary};
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};

pub(super) async fn check_relock(summary: &mut DoctorSummary) {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        println!("wayland.session_lock_restore=unknown");
        summary.record("relock", CheckStatus::Ok, "no portable relock capability query; Sweets, sway and niri replacement behavior is recorded in developer tests");
        return;
    }
    let mut command = Command::new("hyprctl");
    command.args(["-j", "getoption", "misc:allow_session_lock_restore"]);
    match probe(command, Duration::from_secs(2)).await {
        Ok(true) => {
            println!("wayland.session_lock_restore=enabled");
            summary.record(
                "relock",
                CheckStatus::Ok,
                "Hyprland allows a replacement locker after a crash",
            );
        }
        Ok(false) => {
            println!("wayland.session_lock_restore=disabled");
            summary.record("relock", CheckStatus::Warning, "enable Hyprland misc:allow_session_lock_restore to allow emergency recovery after a locker crash");
        }
        Err(error) => {
            println!("wayland.session_lock_restore=unknown");
            summary.record("relock", CheckStatus::Warning, error);
        }
    }
}

async fn probe(mut command: Command, timeout: Duration) -> Result<bool, String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|_| "Hyprland relock query unavailable")?;
    let stdout = child.stdout.take().ok_or("missing Hyprland query output")?;
    let result = tokio::time::timeout(timeout, async {
        let mut bytes = Vec::new();
        stdout.take(4097).read_to_end(&mut bytes).await?;
        let status = child.wait().await?;
        Ok::<_, std::io::Error>((status.success(), bytes))
    })
    .await
    .map_err(|_| "Hyprland relock query timed out")?
    .map_err(|_| "Hyprland relock query failed")?;
    if !result.0 || result.1.len() > 4096 {
        return Err("Hyprland relock query failed or exceeded output limit".into());
    }
    parse(&result.1)
        .ok_or_else(|| "unrecognized Hyprland relock option; check compositor documentation".into())
}

fn parse(bytes: &[u8]) -> Option<bool> {
    #[derive(serde::Deserialize)]
    struct OptionValue {
        int: i64,
    }
    match serde_json::from_slice::<OptionValue>(bytes).ok()?.int {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_boolean_option_values_report_capability() {
        assert_eq!(
            parse(br#"{"option":"misc:allow_session_lock_restore","int":1}"#),
            Some(true)
        );
        assert_eq!(parse(br#"{"int":0}"#), Some(false));
        for bytes in [
            b"{}".as_slice(),
            b"invalid",
            br#"{"int":2}"#,
            br#"{"int":"1"}"#,
        ] {
            assert_eq!(parse(bytes), None);
        }
    }
    #[tokio::test]
    async fn query_times_out_instead_of_hanging_doctor() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "exec sleep 10"]);
        assert!(
            probe(command, Duration::from_millis(20))
                .await
                .unwrap_err()
                .contains("timed out")
        );
    }
    #[tokio::test]
    async fn failed_or_oversized_query_does_not_report_enabled() {
        for script in ["printf '{\"int\":1}'; exit 1", "head -c 4097 /dev/zero"] {
            let mut command = Command::new("/bin/sh");
            command.args(["-c", script]);
            assert!(probe(command, Duration::from_secs(2)).await.is_err());
        }
    }
}
