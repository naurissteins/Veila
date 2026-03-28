#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryIcon {
    Charging,
    Full,
    Medium,
    Low,
    Warning,
}

pub(super) fn battery_svg(icon: BatteryIcon) -> &'static [u8] {
    match icon {
        BatteryIcon::Charging => {
            include_bytes!("../../../../../assets/icons/battery/battery-charging.svg")
        }
        BatteryIcon::Full => {
            include_bytes!("../../../../../assets/icons/battery/battery-full.svg")
        }
        BatteryIcon::Medium => {
            include_bytes!("../../../../../assets/icons/battery/battery-medium.svg")
        }
        BatteryIcon::Low => include_bytes!("../../../../../assets/icons/battery/battery-low.svg"),
        BatteryIcon::Warning => {
            include_bytes!("../../../../../assets/icons/battery/battery-warning.svg")
        }
    }
}
