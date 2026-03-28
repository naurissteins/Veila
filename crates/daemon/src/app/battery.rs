use std::time::Duration;

use tokio::sync::watch;
use veila_common::{BatteryConfig, BatterySnapshot};

use crate::adapters::power;

#[derive(Clone)]
pub(super) struct BatteryHandle {
    config_tx: watch::Sender<BatteryConfig>,
    snapshot_rx: watch::Receiver<Option<BatterySnapshot>>,
}

impl BatteryHandle {
    pub(super) fn spawn(config: &BatteryConfig) -> Self {
        let initial_snapshot = config.mock_snapshot();
        let (config_tx, config_rx) = watch::channel(config.clone());
        let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot);

        tokio::spawn(async move {
            run_battery_service(config_rx, snapshot_tx).await;
        });

        Self {
            config_tx,
            snapshot_rx,
        }
    }

    pub(super) fn current_snapshot(&self) -> Option<BatterySnapshot> {
        self.snapshot_rx.borrow().clone()
    }

    pub(super) fn update_config(&self, config: &BatteryConfig) {
        let _ = self.config_tx.send(config.clone());
    }
}

async fn run_battery_service(
    mut config_rx: watch::Receiver<BatteryConfig>,
    snapshot_tx: watch::Sender<Option<BatterySnapshot>>,
) {
    let mut config = config_rx.borrow().clone();
    let mut needs_refresh = true;

    loop {
        if config.enabled {
            if needs_refresh {
                snapshot_tx.send_replace(fetch_snapshot_async(config.clone()).await);
            }

            let refresh = tokio::time::sleep(refresh_interval(&config));
            tokio::pin!(refresh);

            tokio::select! {
                _ = &mut refresh => {
                    needs_refresh = true;
                }
                changed = config_rx.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    config = config_rx.borrow().clone();
                    snapshot_tx.send_replace(config.mock_snapshot());
                    needs_refresh = true;
                }
            }
        } else {
            snapshot_tx.send_replace(None);
            if config_rx.changed().await.is_err() {
                break;
            }
            config = config_rx.borrow().clone();
            needs_refresh = true;
        }
    }
}

async fn fetch_snapshot_async(config: BatteryConfig) -> Option<BatterySnapshot> {
    if let Some(snapshot) = config.mock_snapshot() {
        tracing::debug!(
            percent = snapshot.percent,
            charging = snapshot.charging,
            "using mock battery snapshot from config"
        );
        return Some(snapshot);
    }

    match power::fetch_battery_snapshot().await {
        Ok(snapshot) => {
            if let Some(snapshot) = snapshot.as_ref() {
                tracing::debug!(
                    percent = snapshot.percent,
                    charging = snapshot.charging,
                    "battery refresh succeeded"
                );
            } else {
                tracing::debug!("no battery device detected; battery widget stays hidden");
            }
            snapshot
        }
        Err(error) => {
            tracing::warn!("battery refresh failed: {error:#}");
            None
        }
    }
}

fn refresh_interval(config: &BatteryConfig) -> Duration {
    Duration::from_secs(u64::from(config.refresh_seconds.max(15)))
}
