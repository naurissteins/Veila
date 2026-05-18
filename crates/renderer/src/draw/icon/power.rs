#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerIcon {
    Suspend,
    Reboot,
    Poweroff,
}

pub(super) fn power_svg(icon: PowerIcon) -> &'static [u8] {
    match icon {
        PowerIcon::Suspend => include_bytes!("../../../../../assets/icons/power/suspend.svg"),
        PowerIcon::Reboot => include_bytes!("../../../../../assets/icons/power/reboot.svg"),
        PowerIcon::Poweroff => include_bytes!("../../../../../assets/icons/power/power-off.svg"),
    }
}
