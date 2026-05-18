use veila_common::PowerAction;
use veila_renderer::{
    FrameSize, PixelBuffer,
    icon::{AssetIcon, PowerIcon},
    shape::Rect,
};

use super::super::{ShellState, ShellStatus};
use super::{
    styles,
    widgets::{draw_chip_block, draw_icon_chip, top_right_chip_diameter},
};

impl ShellState {
    pub(super) fn render_top_right_indicators(&self, buffer: &mut impl PixelBuffer) {
        self.render_power_buttons(buffer);

        let power_block = (self.theme.power_status_enabled
            && matches!(self.status, ShellStatus::Idle))
        .then_some(self.power_status_text.as_deref())
        .flatten()
        .map(|text| {
            self.text_layout_cache.borrow_mut().power_status_block(
                text,
                self.keyboard_layout_text_style(),
                280,
            )
        });
        let keyboard_block = if self.theme.keyboard_enabled {
            self.keyboard_layout_label.as_deref().map(|label| {
                self.text_layout_cache.borrow_mut().keyboard_layout_block(
                    label,
                    self.keyboard_layout_text_style(),
                    120,
                )
            })
        } else {
            None
        };
        let keyboard_chip_diameter = keyboard_block.as_ref().map(|block| {
            top_right_chip_diameter(
                self.theme.keyboard_background_size,
                block.width as i32,
                block.height as i32,
            )
        });

        if let Some(block) = power_block.as_ref()
            && let Some(position) = self.theme.power_status_position
        {
            let chip_diameter = top_right_chip_diameter(
                self.theme.keyboard_background_size,
                block.width as i32,
                block.height as i32,
            );
            let rect = self.positioned_rect(buffer.size(), position, chip_diameter, chip_diameter);
            draw_chip_block(
                buffer,
                rect.x,
                rect.y,
                self.theme.keyboard_background_color,
                self.theme.keyboard_background_size,
                self.theme.keyboard_radius,
                block,
            );
        }

        if let Some(block) = keyboard_block.as_ref()
            && let Some(position) = self.theme.keyboard_position
        {
            let chip_diameter = keyboard_chip_diameter.unwrap_or_else(|| {
                top_right_chip_diameter(
                    self.theme.keyboard_background_size,
                    block.width as i32,
                    block.height as i32,
                )
            });
            let rect = self.positioned_rect(buffer.size(), position, chip_diameter, chip_diameter);
            draw_chip_block(
                buffer,
                rect.x,
                rect.y,
                self.theme.keyboard_background_color,
                self.theme.keyboard_background_size,
                self.theme.keyboard_radius,
                block,
            );
        }

        if self.theme.battery_enabled
            && let Some(battery) = self.battery.as_ref()
            && let Some(position) = self.theme.battery_position
        {
            let battery_icon_size = self.theme.battery_size.unwrap_or(18).clamp(12, 96);
            let chip_diameter = top_right_chip_diameter(
                self.theme.battery_background_size,
                battery_icon_size,
                battery_icon_size,
            );
            let rect = self.positioned_rect(buffer.size(), position, chip_diameter, chip_diameter);
            let battery_color = self.theme.battery_color.unwrap_or(self.theme.foreground);
            let icon_style =
                veila_renderer::icon::IconStyle::new(if battery_color.alpha == u8::MAX {
                    battery_color.with_alpha(styles::percent_to_alpha(68))
                } else {
                    battery_color
                });
            draw_icon_chip(
                buffer,
                rect.x,
                rect.y,
                self.theme.battery_background_color,
                self.theme.battery_background_size,
                self.theme.battery_radius,
                AssetIcon::Battery(battery.icon),
                icon_style,
                battery_icon_size,
            );
        }
    }

    fn render_power_buttons(&self, buffer: &mut impl PixelBuffer) {
        for button in self.theme.power_buttons {
            if !button.enabled {
                continue;
            }

            let Some(rect) = self.power_button_rect(buffer.size(), button.action) else {
                continue;
            };
            let mut button_color = button.color.unwrap_or(self.theme.foreground);
            if self.power_confirmation_action() == Some(button.action) {
                button_color = self.theme.pending;
            }
            let icon_style = veila_renderer::icon::IconStyle::new(button_color);
            draw_icon_chip(
                buffer,
                rect.x,
                rect.y,
                button.background_color,
                button.background_size,
                button.radius,
                AssetIcon::Power(power_icon(button.action)),
                icon_style,
                button.size,
            );
        }
    }

    pub(crate) fn power_button_rect(&self, size: FrameSize, action: PowerAction) -> Option<Rect> {
        let button = self
            .theme
            .power_buttons
            .iter()
            .find(|button| button.action == action && button.enabled)?;
        let position = button.position?;
        let chip_diameter =
            top_right_chip_diameter(button.background_size, button.size, button.size);
        Some(self.positioned_rect(size, position, chip_diameter, chip_diameter))
    }
}

fn power_icon(action: PowerAction) -> PowerIcon {
    match action {
        PowerAction::Suspend => PowerIcon::Suspend,
        PowerAction::Reboot => PowerIcon::Reboot,
        PowerAction::Poweroff => PowerIcon::Poweroff,
    }
}
