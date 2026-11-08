use crate::adapters::logind;
use anyhow::{Result, anyhow};
use veila_common::{PowerAction, config::VisualConfig};

pub fn validate_power_action(visuals: &VisualConfig, action: PowerAction) -> Result<()> {
    // The orchestration owner validates config independently of curtain UI visibility.
    if !visuals.power_button_enabled(action) {
        return Err(anyhow!("power action {action:?} is disabled"));
    }
    Ok(())
}

pub async fn request_power_action(
    manager_proxy: &logind::ManagerProxy<'_>,
    action: PowerAction,
) -> Result<()> {
    tracing::info!(?action, "received mediated power action request");
    match action {
        PowerAction::Suspend => manager_proxy.suspend(false).await?,
        PowerAction::Reboot => manager_proxy.reboot(false).await?,
        PowerAction::Poweroff => manager_proxy.power_off(false).await?,
    }
    Ok(())
}
