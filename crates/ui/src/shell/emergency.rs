use veila_renderer::{
    ClearColor, FrameSize, PixelBuffer,
    masked::{MaskedInputStyle, draw_masked_input},
    shape::{BorderStyle, PillStyle, Rect, draw_pill},
    text::{TextStyle, fit_single_line_text},
};

use super::{ShellState, ShellStatus};

const BACKGROUND: ClearColor = ClearColor::opaque(12, 14, 18);
const FOREGROUND: ClearColor = ClearColor::rgba(244, 247, 251, 242);
const MUTED: ClearColor = ClearColor::rgba(179, 188, 202, 190);
const INPUT: ClearColor = ClearColor::rgba(31, 35, 43, 238);
const BORDER: ClearColor = ClearColor::rgba(150, 164, 184, 184);
const PENDING: ClearColor = ClearColor::rgba(147, 197, 253, 226);
const REJECTED: ClearColor = ClearColor::rgba(248, 113, 113, 230);

impl ShellState {
    pub fn render_emergency(&self, buffer: &mut impl PixelBuffer) {
        buffer.clear(BACKGROUND);
        self.render_emergency_overlay(buffer);
    }

    pub fn render_emergency_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_emergency_scale(scale, |shell| shell.render_emergency(buffer));
    }

    pub fn render_emergency_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_emergency_static_overlay(buffer);
        self.render_emergency_dynamic_overlay(buffer);
    }

    pub fn render_emergency_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_emergency_scale(scale, |shell| {
            shell.render_emergency_overlay(buffer);
        });
    }

    pub fn render_emergency_static_overlay(&self, buffer: &mut impl PixelBuffer) {
        let layout = EmergencyLayout::new(buffer.size(), self.render_scale.max(1));
        let title = fit_single_line_text(
            "Unlock",
            TextStyle::new_px(FOREGROUND, 28 * self.render_scale.max(1)).with_line_spacing(0),
            layout.input.width as u32,
        );
        let hint = fit_single_line_text(
            "Emergency unlock mode",
            TextStyle::new_px(MUTED, 15 * self.render_scale.max(1)).with_line_spacing(0),
            layout.input.width as u32,
        );

        title.draw(
            buffer,
            layout.center_x - title.width as i32 / 2,
            layout.title_y,
        );
        hint.draw(
            buffer,
            layout.center_x - hint.width as i32 / 2,
            layout.hint_y,
        );
        draw_pill(buffer, layout.input, self.emergency_input_style());
    }

    pub fn render_emergency_static_overlay_scaled(
        &self,
        buffer: &mut impl PixelBuffer,
        scale: u32,
    ) {
        self.with_emergency_scale(scale, |shell| {
            shell.render_emergency_static_overlay(buffer);
        });
    }

    pub fn render_emergency_dynamic_overlay(&self, buffer: &mut impl PixelBuffer) {
        let layout = EmergencyLayout::new(buffer.size(), self.render_scale.max(1));
        self.render_emergency_input_content(buffer, layout.input);

        if let Some(text) = self.emergency_status_text() {
            let block = fit_single_line_text(
                &text,
                TextStyle::new_px(self.emergency_status_color(), 15 * self.render_scale.max(1))
                    .with_line_spacing(0),
                layout.input.width as u32,
            );
            block.draw(
                buffer,
                layout.center_x - block.width as i32 / 2,
                layout.status_y,
            );
        }
    }

    pub fn render_emergency_dynamic_overlay_scaled(
        &self,
        buffer: &mut impl PixelBuffer,
        scale: u32,
    ) {
        self.with_emergency_scale(scale, |shell| {
            shell.render_emergency_dynamic_overlay(buffer);
        });
    }

    fn with_emergency_scale(&self, scale: u32, render: impl FnOnce(&ShellState)) {
        let scale = scale.max(1);
        if scale == 1 {
            render(self);
            return;
        }

        let mut scaled = self.clone();
        scaled.render_scale = scale;
        render(&scaled);
    }

    fn render_emergency_input_content(&self, buffer: &mut impl PixelBuffer, rect: Rect) {
        if self.secret.is_empty() {
            let placeholder = fit_single_line_text(
                "Password",
                TextStyle::new_px(MUTED, 16 * self.render_scale.max(1)).with_line_spacing(0),
                rect.width.saturating_sub(48) as u32,
            );
            placeholder.draw(
                buffer,
                rect.x + scaled(22, self.render_scale),
                rect.y + (rect.height - placeholder.height as i32) / 2 - 1,
            );
            return;
        }

        draw_masked_input(
            buffer,
            Rect::new(rect.x, rect.y, rect.width, rect.height),
            self.secret.char_count(),
            self.focused,
            self.emergency_mask_style(),
        );
    }

    fn emergency_input_style(&self) -> PillStyle {
        let border = if matches!(self.status, ShellStatus::Rejected { .. }) {
            REJECTED
        } else if self.focused {
            BORDER
        } else {
            BORDER.with_alpha(128)
        };

        PillStyle::new(INPUT)
            .with_radius(scaled(16, self.render_scale))
            .with_border(BorderStyle::new(border, scaled(2, self.render_scale)))
    }

    fn emergency_mask_style(&self) -> MaskedInputStyle {
        let mut style = MaskedInputStyle::new(FOREGROUND);
        let scale = self.render_scale.max(1) as i32;
        style.bullet_size = style.bullet_size.saturating_mul(scale);
        style.spacing = style.spacing.saturating_mul(scale);
        style.horizontal_padding = scaled(22, self.render_scale);
        style
    }

    fn emergency_status_text(&self) -> Option<String> {
        match &self.status {
            ShellStatus::Idle => None,
            ShellStatus::Pending { shown, .. } => shown.then(|| String::from("Checking...")),
            ShellStatus::Rejected {
                displayed_retry_seconds,
                ..
            } => match displayed_retry_seconds {
                Some(seconds) if *seconds > 0 => Some(format!("Try again in {seconds}s")),
                _ => Some(String::from("Authentication failed")),
            },
        }
    }

    fn emergency_status_color(&self) -> ClearColor {
        match self.status {
            ShellStatus::Pending { .. } => PENDING,
            ShellStatus::Rejected { .. } => REJECTED,
            ShellStatus::Idle => MUTED,
        }
    }
}

struct EmergencyLayout {
    center_x: i32,
    title_y: i32,
    hint_y: i32,
    status_y: i32,
    input: Rect,
}

impl EmergencyLayout {
    fn new(size: FrameSize, scale: u32) -> Self {
        let scale = scale.max(1);
        let width =
            (size.width as i32 - scaled(64, scale)).clamp(scaled(260, scale), scaled(440, scale));
        let height = scaled(56, scale);
        let center_x = size.width as i32 / 2;
        let center_y = size.height as i32 / 2;
        let input_y = center_y - height / 2 + scaled(22, scale);

        Self {
            center_x,
            title_y: input_y - scaled(72, scale),
            hint_y: input_y - scaled(34, scale),
            status_y: input_y + height + scaled(18, scale),
            input: Rect::new(center_x - width / 2, input_y, width, height),
        }
    }
}

fn scaled(value: i32, scale: u32) -> i32 {
    value.saturating_mul(scale.max(1) as i32)
}
