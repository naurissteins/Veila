use anyhow::{Context, Result};
use zbus::{OwnedObjectPath, proxy};

use veila_common::BatterySnapshot;

pub(crate) struct BatteryReading {
    pub snapshot: BatterySnapshot,
    pub on_battery: Option<bool>,
}

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait PowerManager {
    fn get_display_device(&self) -> zbus::Result<OwnedObjectPath>;

    #[zbus(property)]
    fn on_battery(&self) -> zbus::Result<bool>;
}

#[proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower"
)]
trait PowerDevice {
    #[zbus(property, name = "IsPresent")]
    fn is_present(&self) -> zbus::Result<bool>;

    #[zbus(property, name = "Percentage")]
    fn percentage(&self) -> zbus::Result<f64>;

    #[zbus(property, name = "State")]
    fn state(&self) -> zbus::Result<u32>;

    #[zbus(property, name = "Type")]
    fn kind(&self) -> zbus::Result<u32>;
}

pub(crate) async fn fetch_battery_reading(
    read_power_source: bool,
) -> Result<Option<BatteryReading>> {
    let connection = zbus::Connection::system()
        .await
        .context("failed to connect to the system D-Bus for battery state")?;
    let manager = PowerManagerProxy::new(&connection)
        .await
        .context("failed to create UPower manager proxy")?;
    let device_path = manager
        .get_display_device()
        .await
        .context("failed to resolve UPower display device")?;
    let device = PowerDeviceProxy::builder(&connection)
        .path(device_path.as_str())
        .build()
        .await
        .context("failed to create UPower display device proxy")?;

    let is_present = device
        .is_present()
        .await
        .context("failed to read UPower battery presence")?;
    let kind = device
        .kind()
        .await
        .context("failed to read UPower battery type")?;
    if !is_present || kind != 2 {
        return Ok(None);
    }

    let percent = device
        .percentage()
        .await
        .context("failed to read UPower battery percentage")?
        .round()
        .clamp(0.0, 100.0) as u8;
    let state = device
        .state()
        .await
        .context("failed to read UPower battery state")?;

    let snapshot = BatterySnapshot {
        percent,
        charging: matches!(state, 1 | 5),
    };
    let on_battery = if read_power_source {
        match manager.on_battery().await {
            Ok(on_battery) => Some(on_battery),
            Err(error) => {
                tracing::warn!(%error, "failed to read UPower system power source");
                None
            }
        }
    } else {
        None
    };

    Ok(Some(BatteryReading {
        snapshot,
        on_battery,
    }))
}
