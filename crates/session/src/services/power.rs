use crate::adapters::logind;
use anyhow::Result;
use veila_common::PowerAction;

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
