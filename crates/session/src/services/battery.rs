use anyhow::Context;
use std::time::Duration;
use tokio::sync::watch;
use veila_common::{BatteryConfig, BatterySnapshot};
use zbus::Connection;

use crate::adapters::power::{BatteryReading, fetch_battery_reading};

#[derive(Clone)]
pub struct BatteryHandle {
    config_tx: watch::Sender<BatteryServiceConfig>,
    snapshot_rx: watch::Receiver<Option<BatteryReading>>,
}

impl BatteryHandle {
    pub fn spawn(config: &BatteryConfig, read_power_source: bool) -> Self {
        let initial_snapshot = mock_reading(config);
        let (config_tx, config_rx) = watch::channel(BatteryServiceConfig {
            battery: config.clone(),
            read_power_source,
        });
        let (snapshot_tx, snapshot_rx) = watch::channel(initial_snapshot);

        tokio::spawn(async move {
            run_battery_service(config_rx, snapshot_tx).await;
        });

        Self {
            config_tx,
            snapshot_rx,
        }
    }

    pub fn current_snapshot(&self) -> Option<BatterySnapshot> {
        self.snapshot_rx
            .borrow()
            .as_ref()
            .map(|reading| reading.snapshot.clone())
    }

    pub fn on_battery(&self) -> Option<bool> {
        self.snapshot_rx
            .borrow()
            .as_ref()
            .and_then(|reading| reading.on_battery)
    }

    pub fn update_config(&self, config: &BatteryConfig, read_power_source: bool) {
        let _ = self.config_tx.send(BatteryServiceConfig {
            battery: config.clone(),
            read_power_source,
        });
    }
}

#[derive(Clone)]
struct BatteryServiceConfig {
    battery: BatteryConfig,
    read_power_source: bool,
}

async fn run_battery_service(
    mut config_rx: watch::Receiver<BatteryServiceConfig>,
    snapshot_tx: watch::Sender<Option<BatteryReading>>,
) {
    let mut config = config_rx.borrow().clone();
    let mut needs_refresh = true;
    let mut connection = None;

    loop {
        if config.battery.enabled || config.read_power_source {
            if needs_refresh {
                snapshot_tx.send_replace(fetch_snapshot_async(&config, &mut connection).await);
            }

            let refresh = tokio::time::sleep(refresh_interval(&config.battery));
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
                    snapshot_tx.send_replace(mock_reading(&config.battery));
                    needs_refresh = true;
                }
            }
        } else {
            connection = None;
            snapshot_tx.send_replace(None);
            if config_rx.changed().await.is_err() {
                break;
            }
            config = config_rx.borrow().clone();
            needs_refresh = true;
        }
    }
}

async fn fetch_snapshot_async(
    config: &BatteryServiceConfig,
    connection: &mut Option<Connection>,
) -> Option<BatteryReading> {
    if let Some(reading) = mock_reading(&config.battery) {
        *connection = None;
        tracing::debug!(
            percent = reading.snapshot.percent,
            charging = reading.snapshot.charging,
            "using mock battery snapshot from config"
        );
        return Some(reading);
    }

    let result = async {
        let connection = match connection {
            Some(connection) => connection,
            None => connection.insert(
                Connection::system()
                    .await
                    .context("failed to connect to the system D-Bus for battery state")?,
            ),
        };
        fetch_battery_reading(connection, config.read_power_source).await
    }
    .await;

    match result {
        Ok(Some(reading)) => {
            tracing::debug!(
                percent = reading.snapshot.percent,
                charging = reading.snapshot.charging,
                on_battery = ?reading.on_battery,
                "battery refresh succeeded"
            );
            Some(reading)
        }
        Ok(None) => {
            tracing::debug!("no battery device detected; battery widget stays hidden");
            None
        }
        Err(error) => {
            *connection = None;
            tracing::warn!("battery refresh failed: {error:#}");
            None
        }
    }
}

fn mock_reading(config: &BatteryConfig) -> Option<BatteryReading> {
    let snapshot = config.mock_snapshot()?;
    Some(BatteryReading {
        on_battery: Some(!snapshot.charging),
        snapshot,
    })
}

fn refresh_interval(config: &BatteryConfig) -> Duration {
    Duration::from_secs(u64::from(config.refresh_seconds.max(15)))
}
