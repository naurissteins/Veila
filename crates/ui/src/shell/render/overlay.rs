use std::cell::RefCell;

use veila_renderer::{
    FrameSize, PixelBuffer,
    shape::{Rect, fill_rect},
};

use super::super::{ShellState, ShellStatus};
use super::{
    SceneLayout, TextLayoutCache,
    layout::{SceneMetrics, hero_block_x},
    model::{AuthGroup, LayoutRole, SceneSection, SceneWidget},
    widgets::{
        InputRightAdornment, InputWidget, draw_avatar_widget, draw_block, draw_centered_block,
        draw_clock_widget, draw_input_content, draw_input_shell, draw_weather_icon,
        input_toggle_hitbox,
    },
};

impl ShellState {
    pub fn render(&self, buffer: &mut impl PixelBuffer) {
        buffer.clear(self.theme.background);
        self.render_overlay(buffer);
    }

    pub fn render_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |shell| shell.render(buffer));
    }

    pub fn render_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_backdrops(buffer);
        self.render_static_overlay(buffer);
        self.render_dynamic_overlay(buffer);
    }

    pub fn render_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |shell| shell.render_overlay(buffer));
    }

    pub fn render_static_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_layers(buffer);
        let layout = self.scene_layout(buffer.size());
        self.render_identity_group(buffer, &layout, false);
        self.render_floating_identity_widgets(buffer, &layout);
        self.render_floating_input_widgets(buffer, &layout, false);
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Hero,
            layout.anchors.hero_y,
            false,
        );
        self.render_auth_or_input_group(buffer, &layout, false);
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Footer,
            layout.anchors.footer_y,
            false,
        );
    }

    pub fn render_static_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |shell| shell.render_static_overlay(buffer));
    }

    pub fn render_dynamic_overlay(&self, buffer: &mut impl PixelBuffer) {
        let layout = self.scene_layout(buffer.size());
        self.render_identity_group(buffer, &layout, true);
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Hero,
            layout.anchors.hero_y,
            true,
        );
        self.render_auth_or_input_group(buffer, &layout, true);
        self.render_role(
            buffer,
            &layout,
            LayoutRole::Footer,
            layout.anchors.footer_y,
            true,
        );
        self.render_floating_header_widgets(buffer, &layout);
        self.render_floating_input_widgets(buffer, &layout, true);
        self.render_floating_weather_widgets(buffer, &layout);
        self.render_now_playing_widget(buffer, &layout);
        self.render_top_right_indicators(buffer);
        self.render_preview_grid_overlay(buffer);
    }

    pub fn render_dynamic_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |shell| shell.render_dynamic_overlay(buffer));
    }

    fn with_render_scale(&self, scale: u32, render: impl FnOnce(&ShellState)) {
        let scale = scale.max(1);
        if scale == 1 {
            render(self);
            return;
        }

        let mut scaled = self.clone();
        scaled.render_scale = scale;
        scaled.theme = self.theme.scaled_for_render(scale);
        scaled.text_layout_cache = RefCell::new(TextLayoutCache::default());
        render(&scaled);
    }

    fn render_role(
        &self,
        buffer: &mut impl PixelBuffer,
        layout: &SceneLayout,
        role: LayoutRole,
        start_y: i32,
        dynamic: bool,
    ) {
        let mut y = start_y;

        for section in layout.model.sections_for_role(role) {
            self.render_section(buffer, layout.metrics, section, y, dynamic);
            y += section.height(layout.metrics, &self.status) + section.gap_after;
        }
    }

    fn render_identity_group(
        &self,
        buffer: &mut impl PixelBuffer,
        layout: &SceneLayout,
        dynamic: bool,
    ) {
        let Some(start_y) = layout.anchors.identity_y else {
            return;
        };

        let mut y = start_y;
        for section in layout.model.sections_for_auth_group(AuthGroup::Identity) {
            self.render_section(buffer, layout.metrics, section, y, dynamic);
            y += section.height(layout.metrics, &self.status) + section.gap_after;
        }
    }

    fn render_auth_or_input_group(
        &self,
        buffer: &mut impl PixelBuffer,
        layout: &SceneLayout,
        dynamic: bool,
    ) {
        if layout.anchors.identity_y.is_some() {
            let mut y = layout.anchors.auth_y;
            for section in layout.model.sections_for_auth_group(AuthGroup::Input) {
                self.render_section(buffer, layout.metrics, section, y, dynamic);
                y += section.height(layout.metrics, &self.status) + section.gap_after;
            }
        } else {
            self.render_role(
                buffer,
                layout,
                LayoutRole::Auth,
                layout.anchors.auth_y,
                dynamic,
            );
        }
    }

    fn render_floating_header_widgets(&self, buffer: &mut impl PixelBuffer, layout: &SceneLayout) {
        if let Some(clock) = layout.floating_clock.as_ref() {
            let position = self
                .theme
                .clock_position
                .expect("floating clock requires explicit position");
            let rect = self.positioned_rect(buffer.size(), position, clock.width(), clock.height());
            draw_clock_widget(buffer, rect.x, rect.y, clock);
        }

        if let Some(date) = layout.floating_date.as_ref() {
            let position = self
                .theme
                .date_position
                .expect("floating date requires resolved position");
            let rect = self.positioned_rect(
                buffer.size(),
                position,
                date.width as i32,
                date.height as i32,
            );
            draw_block(buffer, rect.x, rect.y, date);
        }
    }

    fn render_floating_identity_widgets(
        &self,
        buffer: &mut impl PixelBuffer,
        layout: &SceneLayout,
    ) {
        if layout.floating_avatar {
            let position = self
                .theme
                .avatar_position
                .expect("floating avatar requires explicit position");
            let size = layout.metrics.avatar_size;
            let rect = self.positioned_rect(buffer.size(), position, size, size);
            draw_avatar_widget(
                buffer,
                &self.avatar,
                rect.x + size / 2,
                rect.y,
                size as u32,
                self.avatar_style(),
            );
        }

        if let Some(username) = layout.floating_username.as_ref() {
            let position = self
                .theme
                .username_position
                .expect("floating username requires resolved position");
            let rect = self.positioned_rect(
                buffer.size(),
                position,
                username.width as i32,
                username.height as i32,
            );
            draw_block(buffer, rect.x, rect.y, username);
        }
    }

    fn render_floating_input_widgets(
        &self,
        buffer: &mut impl PixelBuffer,
        layout: &SceneLayout,
        dynamic: bool,
    ) {
        if layout.floating_input {
            let rect = self
                .floating_input_rect(layout, buffer.size())
                .expect("floating input requires explicit position");
            let placeholder = layout.floating_input_placeholder.clone();
            self.render_input_widget(buffer, rect, placeholder, dynamic);
        }

        if dynamic && let Some(status) = layout.floating_status.as_ref() {
            let (x, y) = self
                .floating_status_origin(layout, buffer.size(), status)
                .expect("floating status requires explicit origin");
            draw_block(buffer, x, y, status);
        }
    }

    fn render_floating_weather_widgets(&self, buffer: &mut impl PixelBuffer, layout: &SceneLayout) {
        let Some(weather) = layout.floating_weather.as_ref() else {
            return;
        };

        if let Some(icon) = weather.icon
            && let Some(position) = self.theme.weather_icon_position
        {
            let rect = self.positioned_rect(buffer.size(), position, icon.size, icon.size);
            draw_weather_icon(buffer, rect.x, rect.y, icon.asset, icon.size, icon.opacity);
        }

        if let Some(temperature) = weather.temperature.as_ref()
            && let Some(position) = self.theme.weather_temperature_position
        {
            let rect = self.positioned_rect(
                buffer.size(),
                position,
                temperature.width as i32,
                temperature.height as i32,
            );
            draw_block(buffer, rect.x, rect.y, temperature);
        }

        if let Some(location) = weather.location.as_ref()
            && let Some(position) = self.theme.weather_location_position
        {
            let rect = self.positioned_rect(
                buffer.size(),
                position,
                location.width as i32,
                location.height as i32,
            );
            draw_block(buffer, rect.x, rect.y, location);
        }
    }

    fn render_preview_grid_overlay(&self, buffer: &mut impl PixelBuffer) {
        if !self.preview_grid_enabled {
            return;
        }

        let Some(grid) = self.theme.grid else {
            return;
        };

        let width = buffer.size().width as i32;
        let height = buffer.size().height as i32;
        let center_x = width / 2;
        let center_y = height / 2;

        let mut x = center_x;
        let mut index = 0;
        while x < width {
            self.draw_grid_vertical_line(buffer, x, height, index, grid);
            index += 1;
            x += grid.cell_size;
        }

        let mut x = center_x - grid.cell_size;
        let mut index = 1;
        while x >= 0 {
            self.draw_grid_vertical_line(buffer, x, height, index, grid);
            index += 1;
            x -= grid.cell_size;
        }

        let mut y = center_y;
        let mut index = 0;
        while y < height {
            self.draw_grid_horizontal_line(buffer, y, width, index, grid);
            index += 1;
            y += grid.cell_size;
        }

        let mut y = center_y - grid.cell_size;
        let mut index = 1;
        while y >= 0 {
            self.draw_grid_horizontal_line(buffer, y, width, index, grid);
            index += 1;
            y -= grid.cell_size;
        }
    }

    fn draw_grid_vertical_line(
        &self,
        buffer: &mut impl PixelBuffer,
        x: i32,
        height: i32,
        index: i32,
        grid: crate::shell::PreviewGrid,
    ) {
        fill_rect(
            buffer,
            Rect::new(x, 0, 1, height),
            if index % grid.major_every == 0 {
                grid.major_color
            } else {
                grid.color
            },
        );
    }

    fn draw_grid_horizontal_line(
        &self,
        buffer: &mut impl PixelBuffer,
        y: i32,
        width: i32,
        index: i32,
        grid: crate::shell::PreviewGrid,
    ) {
        fill_rect(
            buffer,
            Rect::new(0, y, width, 1),
            if index % grid.major_every == 0 {
                grid.major_color
            } else {
                grid.color
            },
        );
    }

    fn floating_input_rect(&self, layout: &SceneLayout, size: FrameSize) -> Option<Rect> {
        let position = self.theme.input_position?;
        Some(self.positioned_rect(
            size,
            position,
            layout.metrics.input_width,
            layout.metrics.input_height,
        ))
    }

    fn floating_status_origin(
        &self,
        layout: &SceneLayout,
        size: FrameSize,
        block: &veila_renderer::text::TextBlock,
    ) -> Option<(i32, i32)> {
        if let Some(position) = self.theme.status_position {
            let rect =
                self.positioned_rect(size, position, block.width as i32, block.height as i32);
            return Some((rect.x, rect.y));
        }

        if layout.floating_status_follows_input {
            let input_rect = self.floating_input_rect(layout, size)?;
            let x = input_rect.x + (input_rect.width - block.width as i32) / 2;
            let gap = 14;
            let y = if matches!(
                self.theme.input_position?.valign,
                veila_common::VerticalAlign::Bottom
            ) {
                input_rect.y - gap - block.height as i32
            } else {
                input_rect.y + input_rect.height + gap
            };
            return Some((x, y));
        }

        None
    }

    fn render_section(
        &self,
        buffer: &mut impl PixelBuffer,
        metrics: SceneMetrics,
        section: &SceneSection,
        y: i32,
        dynamic: bool,
    ) {
        match &section.widget {
            SceneWidget::Clock(block) if dynamic => {
                let backdrop_center_x = self
                    .theme
                    .clock_center_in_layer
                    .then(|| self.first_backdrop_center_x(buffer.size()))
                    .flatten();
                let x = hero_block_x(
                    buffer.size().width as i32,
                    block.width(),
                    self.theme.clock_alignment,
                    backdrop_center_x,
                    self.theme.clock_offset_x,
                );
                draw_clock_widget(buffer, x, y, block);
            }
            SceneWidget::Date(block) | SceneWidget::Status(block) if dynamic => {
                if matches!(section.widget, SceneWidget::Status(_)) {
                    draw_centered_block(buffer, metrics.auth_center_x, y, block);
                } else {
                    let backdrop_center_x = self
                        .theme
                        .clock_center_in_layer
                        .then(|| self.first_backdrop_center_x(buffer.size()))
                        .flatten();
                    let x = hero_block_x(
                        buffer.size().width as i32,
                        block.width as i32,
                        self.theme.clock_alignment,
                        backdrop_center_x,
                        self.theme.clock_offset_x,
                    );
                    draw_block(buffer, x, y, block);
                }
            }
            SceneWidget::Username(block) if !dynamic => {
                draw_centered_block(
                    buffer,
                    metrics.auth_center_x,
                    y + self.theme.username_offset_y.unwrap_or(0),
                    block,
                );
            }
            SceneWidget::Avatar if !dynamic && self.theme.avatar_enabled => {
                draw_avatar_widget(
                    buffer,
                    &self.avatar,
                    metrics.auth_center_x,
                    y + self.theme.avatar_offset_y.unwrap_or(0),
                    metrics.avatar_size as u32,
                    self.avatar_style(),
                );
            }
            SceneWidget::Input(placeholder) => {
                self.render_input_widget(
                    buffer,
                    metrics.input_rect(y),
                    placeholder.clone(),
                    dynamic,
                );
            }
            _ => {}
        }
    }

    fn render_input_widget(
        &self,
        buffer: &mut impl PixelBuffer,
        rect: Rect,
        placeholder: Option<veila_renderer::text::TextBlock>,
        dynamic: bool,
    ) {
        let revealed_secret = if self.reveal_secret && !self.secret.is_empty() {
            Some(self.text_layout_cache.borrow_mut().revealed_secret_block(
                &self.secret,
                self.revealed_secret_text_style(),
                rect.width.saturating_sub(92) as u32,
            ))
        } else {
            None
        };
        let inline_status = if dynamic {
            self.inline_input_status_text().map(|text| {
                self.text_layout_cache.borrow_mut().input_status_block(
                    &text,
                    self.input_status_text_style(),
                    rect.width.saturating_sub(92) as u32,
                )
            })
        } else {
            None
        };
        let right_adornment = if let Some(phase) = self.pending_spinner_phase() {
            InputRightAdornment::Spinner {
                phase,
                style: self.toggle_style(),
            }
        } else if self.caps_lock_active && self.theme.caps_lock_enabled {
            InputRightAdornment::CapsLock {
                style: self.caps_lock_icon_style(),
            }
        } else if self.theme.eye_enabled {
            InputRightAdornment::Toggle {
                hovered: self.reveal_toggle_hovered,
                pressed: self.reveal_toggle_pressed,
                reveal_secret: self.reveal_secret,
                style: self.toggle_style(),
            }
        } else {
            InputRightAdornment::None
        };
        let widget = InputWidget {
            rect,
            secret_len: self.secret.chars().count(),
            focused: self.focused,
            shell_style: self.input_style(),
            mask_style: self.mask_style(),
            placeholder,
            revealed_secret,
            inline_status,
            right_adornment,
        };
        if dynamic {
            if self.input_shell_is_dynamic() {
                draw_input_shell(buffer, widget.rect, widget.shell_style);
            }
            draw_input_content(buffer, &widget);
        } else if !self.input_shell_is_dynamic() {
            draw_input_shell(buffer, widget.rect, widget.shell_style);
        }
    }

    pub(crate) fn reveal_toggle_rect_for_frame(
        &self,
        frame_width: i32,
        frame_height: i32,
    ) -> veila_renderer::shape::Rect {
        let layout = self.scene_layout(FrameSize::new(
            frame_width.max(1) as u32,
            frame_height.max(1) as u32,
        ));
        if layout.floating_input {
            return if self.theme.eye_enabled {
                self.floating_input_rect(
                    &layout,
                    FrameSize::new(frame_width.max(1) as u32, frame_height.max(1) as u32),
                )
                .map(input_toggle_hitbox)
                .unwrap_or_else(|| veila_renderer::shape::Rect::new(0, 0, 0, 0))
            } else {
                veila_renderer::shape::Rect::new(0, 0, 0, 0)
            };
        }
        let mut y = layout.anchors.auth_y;

        if layout.anchors.identity_y.is_some() {
            for section in layout.model.sections_for_auth_group(AuthGroup::Input) {
                if matches!(section.widget, SceneWidget::Input(_)) {
                    return if self.theme.eye_enabled {
                        input_toggle_hitbox(layout.metrics.input_rect(y))
                    } else {
                        veila_renderer::shape::Rect::new(0, 0, 0, 0)
                    };
                }
                y += section.height(layout.metrics, &self.status) + section.gap_after;
            }
        } else {
            for section in layout.model.sections_for_role(LayoutRole::Auth) {
                if matches!(section.widget, SceneWidget::Input(_)) {
                    return if self.theme.eye_enabled {
                        input_toggle_hitbox(layout.metrics.input_rect(y))
                    } else {
                        veila_renderer::shape::Rect::new(0, 0, 0, 0)
                    };
                }
                y += section.height(layout.metrics, &self.status) + section.gap_after;
            }
        }

        veila_renderer::shape::Rect::new(0, 0, 0, 0)
    }

    pub(crate) fn status_text(&self) -> Option<String> {
        match &self.status {
            ShellStatus::Idle => None,
            ShellStatus::Pending { shown, .. } => {
                shown.then(|| String::from("Checking authentication"))
            }
            ShellStatus::Rejected {
                displayed_retry_seconds,
                failed_attempts,
                ..
            } => Some(rejected_status_text(
                *failed_attempts,
                *displayed_retry_seconds,
            )),
        }
    }

    pub(crate) fn inline_input_status_text(&self) -> Option<String> {
        if !self.input_visible()
            || !self.theme.status_enabled
            || self.theme.status_mode != veila_common::StatusDisplayMode::Inline
        {
            return None;
        }

        match &self.status {
            ShellStatus::Idle => None,
            ShellStatus::Pending { shown, .. } => shown.then(|| String::from("Checking...")),
            ShellStatus::Rejected {
                displayed_retry_seconds,
                ..
            } => match displayed_retry_seconds {
                Some(retry_seconds) if *retry_seconds > 0 => {
                    Some(format!("Try again in {retry_seconds}s"))
                }
                _ => Some(String::from("Authentication failed")),
            },
        }
    }

    fn input_shell_is_dynamic(&self) -> bool {
        self.secret_selected || matches!(self.status, ShellStatus::Rejected { .. })
    }
}

fn rejected_status_text(failed_attempts: Option<u8>, retry_seconds: Option<u64>) -> String {
    let count_text = failed_attempts.map(|failed_attempts| {
        let suffix = if failed_attempts == 1 { "" } else { "s" };
        format!("{failed_attempts} failed attempt{suffix}")
    });

    match (count_text, retry_seconds) {
        (Some(count_text), Some(retry_seconds)) if retry_seconds > 0 => {
            format!("Authentication failed ({count_text}), retry in {retry_seconds}s")
        }
        (Some(count_text), Some(_)) | (Some(count_text), None) => {
            format!("Authentication failed ({count_text})")
        }
        (None, Some(retry_seconds)) if retry_seconds > 0 => {
            format!("Authentication failed, retry in {retry_seconds}s")
        }
        (None, Some(_)) | (None, None) => String::from("Authentication failed"),
    }
}
