use veila_renderer::SoftwareBuffer;

use super::super::ShellState;
use super::{
    layout::top_role_top,
    styles,
    widgets::{draw_top_right_block, draw_top_right_icon_chip, top_right_chip_diameter},
};

impl ShellState {
    pub(super) fn render_top_right_indicators(&self, buffer: &mut SoftwareBuffer) {
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
        let row_gap = self.theme.battery_gap.unwrap_or(8).clamp(0, 64);

        if let Some(block) = keyboard_block.as_ref() {
            let y = (top_role_top(buffer.size().height as i32, self.theme.header_top_offset) - 10
                + self.theme.keyboard_top_offset.unwrap_or(0))
            .max(8);
            draw_top_right_block(
                buffer,
                32,
                self.theme.keyboard_right_offset.unwrap_or(0),
                y,
                self.theme.keyboard_background_color,
                self.theme.keyboard_background_size,
                block,
            );
        }

        if self.theme.battery_enabled
            && let Some(battery) = self.battery.as_ref()
        {
            let battery_icon_size = self.theme.battery_size.unwrap_or(18).clamp(12, 96);
            let right_padding = 32
                + keyboard_chip_diameter.unwrap_or(0)
                + if keyboard_chip_diameter.is_some() {
                    row_gap
                } else {
                    0
                };
            let y = (top_role_top(buffer.size().height as i32, self.theme.header_top_offset) - 10
                + self.theme.battery_top_offset.unwrap_or(0))
            .max(8);
            let icon_style = veila_renderer::icon::IconStyle::new(
                self.theme
                    .battery_color
                    .unwrap_or(self.theme.foreground)
                    .with_alpha(
                        self.theme
                            .battery_opacity
                            .map(styles::percent_to_alpha)
                            .unwrap_or(u8::MAX),
                    ),
            );
            draw_top_right_icon_chip(
                buffer,
                right_padding,
                self.theme.battery_right_offset.unwrap_or(0),
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
