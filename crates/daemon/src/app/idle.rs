use std::time::Duration;

use veila_common::IdleConfig;

use crate::adapters::{idle::IdleNotifier, user_units};

#[derive(Default)]
pub(super) struct IdleMonitor {
    timeout: Option<Duration>,
    notifier: Option<IdleNotifier>,
}

impl IdleMonitor {
    pub(super) fn sync(&mut self, config: &IdleConfig) {
        let timeout = idle_timeout(config);
        if timeout == self.timeout {
            return;
        }

        if config.enabled && !config.lock_after_in_range() {
            tracing::warn!(
                configured_seconds = config.lock_after_seconds,
                effective_seconds = timeout.map(|timeout| timeout.as_secs()),
                "idle.lock_after_seconds is out of range; clamping"
            );
        }
        if timeout.is_none() {
            tracing::info!("idle locking disabled");
        }

        self.timeout = timeout;
        self.notifier = timeout.map(IdleNotifier::spawn);
    }

    /// Resolves when the compositor reports the configured idle time; pending while disabled.
    pub(super) async fn idled(&mut self) {
        while let Some(notifier) = self.notifier.as_mut() {
            if notifier.idled().await.is_some() {
                return;
            }
            // A stopped notifier stays gone until the idle config changes, so failures log once.
            self.notifier = None;
        }
        std::future::pending::<()>().await;
    }
}

pub(super) fn warn_about_legacy_idle_service() {
    let links =
        user_units::enablement_links(&user_units::config_dir(), user_units::LEGACY_IDLE_SERVICE);
    for link in links {
        tracing::warn!(
            link = %link.display(),
            "veila-idle.service was removed and no longer locks on idle; set `enabled = true` under [idle] in config.toml and remove this link"
        );
    }
}

fn idle_timeout(config: &IdleConfig) -> Option<Duration> {
    config
        .effective_lock_after_seconds()
        .map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use veila_common::IdleConfig;

    use super::idle_timeout;

    #[test]
    fn idle_timeout_follows_enabled_flag_and_clamps() {
        let disabled = IdleConfig::default();
        let enabled = IdleConfig {
            enabled: true,
            lock_after_seconds: 120,
            ..IdleConfig::default()
        };
        let too_short = IdleConfig {
            enabled: true,
            lock_after_seconds: 1,
            ..IdleConfig::default()
        };

        assert_eq!(idle_timeout(&disabled), None);
        assert_eq!(idle_timeout(&enabled), Some(Duration::from_secs(120)));
        assert_eq!(idle_timeout(&too_short), Some(Duration::from_secs(5)));
    }

    #[test]
    fn sleep_lock_setting_does_not_affect_idle_timeout() {
        let config = IdleConfig {
            enabled: false,
            lock_before_sleep: true,
            ..IdleConfig::default()
        };

        assert_eq!(idle_timeout(&config), None);
    }
}
