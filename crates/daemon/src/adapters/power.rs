use anyhow::{Context, Result};
use zbus::{proxy, zvariant::OwnedObjectPath};

use veila_common::BatterySnapshot;

#[proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait PowerManager {
    fn get_display_device(&self) -> zbus::Result<OwnedObjectPath>;
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

pub async fn fetch_battery_snapshot() -> Result<Option<BatterySnapshot>> {
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
        .context("invalid UPower display device object path")?
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

    Ok(Some(BatterySnapshot {
        percent,
        charging: matches!(state, 1 | 5),
    }))
}
