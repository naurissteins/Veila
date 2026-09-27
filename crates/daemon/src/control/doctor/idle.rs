use std::path::{Path, PathBuf};

use veila_common::{AppConfig, IdleConfig};

use crate::adapters::user_units::{self, LEGACY_IDLE_SERVICE};

use super::{CheckStatus, DoctorSummary};

pub(super) fn check_idle(
    summary: &mut DoctorSummary,
    config_path: Option<&Path>,
    idle_protocol: Option<bool>,
) {
    let idle = AppConfig::load(config_path)
        .map(|loaded| loaded.config.idle)
        .unwrap_or_default();
    let legacy_links = user_units::enablement_links(&user_units::config_dir(), LEGACY_IDLE_SERVICE);

    println!("idle.enabled={}", idle.enabled);
    println!(
        "idle.lock_after_seconds={}",
        idle.effective_lock_after_seconds()
            .unwrap_or(idle.lock_after_seconds)
    );
    println!("idle.lock_before_sleep={}", idle.lock_before_sleep);
    println!(
        "idle.protocol={}",
        match idle_protocol {
            Some(true) => "advertised",
            Some(false) => "missing",
            None => "unknown",
        }
    );
    if legacy_links.is_empty() {
        println!("idle.legacy_service=none");
    }
    for link in &legacy_links {
        println!("idle.legacy_service={}", link.display());
    }

    let (status, detail) = idle_verdict(&idle, idle_protocol, &legacy_links);
    summary.record("idle", status, detail);
}

fn idle_verdict(
    idle: &IdleConfig,
    idle_protocol: Option<bool>,
    legacy_links: &[PathBuf],
) -> (CheckStatus, String) {
    if !legacy_links.is_empty() {
        let links = legacy_links
            .iter()
            .map(|link| link.display().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let detail = if idle.enabled {
            format!(
                "{LEGACY_IDLE_SERVICE} no longer exists and [idle] already handles idle locking; remove the leftover link with `rm {links}`"
            )
        } else {
            format!(
                "{LEGACY_IDLE_SERVICE} no longer exists, so it no longer locks on idle; set `enabled = true` under [idle] in config.toml, run `veila reload`, then `rm {links}`"
            )
        };
        return (CheckStatus::Warning, detail);
    }

    if idle.enabled && idle_protocol == Some(false) {
        return (
            CheckStatus::Warning,
            "idle locking is enabled but the compositor does not advertise ext-idle-notify-v1"
                .to_string(),
        );
    }

    if idle.enabled && !idle.lock_after_in_range() {
        return (
            CheckStatus::Warning,
            format!(
                "idle.lock_after_seconds = {} is out of range; the daemon uses {} seconds",
                idle.lock_after_seconds,
                idle.effective_lock_after_seconds()
                    .unwrap_or(idle.lock_after_seconds)
            ),
        );
    }

    let idle_state = match idle.effective_lock_after_seconds() {
        Some(seconds) => format!("idle locking after {seconds} seconds"),
        None => "idle locking disabled".to_string(),
    };
    let sleep_state = if idle.lock_before_sleep {
        "lock before sleep on"
    } else {
        "lock before sleep off"
    };
    (CheckStatus::Ok, format!("{idle_state}; {sleep_state}"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use veila_common::IdleConfig;

    use super::{CheckStatus, idle_verdict};

    fn enabled(lock_after_seconds: u64) -> IdleConfig {
        IdleConfig {
            enabled: true,
            lock_after_seconds,
            ..IdleConfig::default()
        }
    }

    #[test]
    fn legacy_idle_service_link_is_a_warning_even_when_idle_is_configured() {
        let links = [PathBuf::from("/tmp/wants/veila-idle.service")];
        let (status, detail) = idle_verdict(&enabled(300), Some(true), &links);

        assert_eq!(status, CheckStatus::Warning);
        assert!(detail.contains("rm /tmp/wants/veila-idle.service"));
    }

    #[test]
    fn missing_protocol_only_warns_when_idle_is_enabled() {
        assert_eq!(
            idle_verdict(&enabled(300), Some(false), &[]).0,
            CheckStatus::Warning
        );
        assert_eq!(
            idle_verdict(&IdleConfig::default(), Some(false), &[]).0,
            CheckStatus::Ok
        );
    }

    #[test]
    fn reports_effective_settings_when_healthy() {
        let (status, detail) = idle_verdict(&enabled(600), Some(true), &[]);

        assert_eq!(status, CheckStatus::Ok);
        assert_eq!(
            detail,
            "idle locking after 600 seconds; lock before sleep on"
        );
        assert_eq!(
            idle_verdict(&enabled(1), Some(true), &[]).0,
            CheckStatus::Warning
        );
    }
}
