use veila_common::BatterySnapshot;
use veila_renderer::icon::BatteryIcon;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BatteryWidgetData {
    pub(super) icon: BatteryIcon,
}

pub(super) fn widget_data(snapshot: Option<BatterySnapshot>) -> Option<BatteryWidgetData> {
    let snapshot = snapshot?;
    let icon = if snapshot.charging {
        BatteryIcon::Charging
    } else if snapshot.percent >= 80 {
        BatteryIcon::Full
    } else if snapshot.percent >= 35 {
        BatteryIcon::Medium
    } else if snapshot.percent >= 15 {
        BatteryIcon::Low
    } else {
        BatteryIcon::Warning
    };

    Some(BatteryWidgetData { icon })
}
