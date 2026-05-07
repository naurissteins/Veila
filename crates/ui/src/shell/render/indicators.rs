use veila_renderer::SoftwareBuffer;

use super::super::{ShellState, ShellStatus};
use super::{
    layout::{anchored_block_x, anchored_block_y},
    styles,
    widgets::{draw_chip_block, draw_icon_chip, top_right_chip_diameter},
};

impl ShellState {
    pub(super) fn render_top_right_indicators(&self, buffer: &mut SoftwareBuffer) {
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
            let x = anchored_block_x(
                buffer.size().width as i32,
                chip_diameter,
                position.halign,
                position.x,
            );
            let y = anchored_block_y(
                buffer.size().height as i32,
                chip_diameter,
                position.valign,
                position.y,
            );
            draw_chip_block(
                buffer,
                x,
                y,
                self.theme.keyboard_background_color,
                self.theme.keyboard_background_size,
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
            let x = anchored_block_x(
                buffer.size().width as i32,
                chip_diameter,
                position.halign,
                position.x,
            );
            let y = anchored_block_y(
                buffer.size().height as i32,
                chip_diameter,
                position.valign,
                position.y,
            );
            draw_chip_block(
                buffer,
                x,
                y,
                self.theme.keyboard_background_color,
                self.theme.keyboard_background_size,
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
            let x = anchored_block_x(
                buffer.size().width as i32,
                chip_diameter,
                position.halign,
                position.x,
            );
            let y = anchored_block_y(
                buffer.size().height as i32,
                chip_diameter,
                position.valign,
                position.y,
            );
            let battery_color = self.theme.battery_color.unwrap_or(self.theme.foreground);
            let icon_style =
                veila_renderer::icon::IconStyle::new(if battery_color.alpha == u8::MAX {
                    battery_color.with_alpha(styles::percent_to_alpha(68))
                } else {
                    battery_color
                });
            draw_icon_chip(
                buffer,
                x,
                y,
                self.theme.battery_background_color,
                self.theme.battery_background_size,
                battery.icon,
                icon_style,
                battery_icon_size,
            );
        }
    }
}
