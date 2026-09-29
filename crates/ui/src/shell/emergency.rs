use veila_renderer::{
    ClearColor, FrameSize, PixelBuffer, RenderScale,
    masked::{MaskedInputStyle, draw_masked_input},
    shape::{BorderStyle, PillStyle, Rect, draw_pill},
    text::{TextStyle, fit_sensitive_single_line_text, fit_single_line_text},
};

use super::{ShellStatus, render::RenderContext};

const BACKGROUND: ClearColor = ClearColor::opaque(12, 14, 18);
const FOREGROUND: ClearColor = ClearColor::rgba(244, 247, 251, 242);
const MUTED: ClearColor = ClearColor::rgba(179, 188, 202, 190);
const INPUT: ClearColor = ClearColor::rgba(31, 35, 43, 238);
const BORDER: ClearColor = ClearColor::rgba(150, 164, 184, 184);
const PENDING: ClearColor = ClearColor::rgba(147, 197, 253, 226);
const REJECTED: ClearColor = ClearColor::rgba(248, 113, 113, 230);

impl RenderContext<'_> {
    pub fn render_emergency(&self, buffer: &mut impl PixelBuffer) {
        buffer.clear(BACKGROUND);
        self.render_emergency_overlay(buffer);
    }

    pub fn render_emergency_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_emergency_static_overlay(buffer);
        self.render_emergency_dynamic_overlay(buffer);
    }

    pub fn render_emergency_static_overlay(&self, buffer: &mut impl PixelBuffer) {
        let layout = EmergencyLayout::new(buffer.size(), self.render_scale);
        let title = fit_single_line_text(
            "Unlock",
            TextStyle::new_px(FOREGROUND, self.render_scale.apply_u32(28)).with_line_spacing(0),
            layout.input.width as u32,
        );
        let hint = fit_single_line_text(
            "Emergency unlock mode",
            TextStyle::new_px(MUTED, self.render_scale.apply_u32(15)).with_line_spacing(0),
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

    pub fn render_emergency_dynamic_overlay(&self, buffer: &mut impl PixelBuffer) {
        let layout = EmergencyLayout::new(buffer.size(), self.render_scale);
        self.render_emergency_input_content(buffer, layout.input);

        if let Some(text) = self.emergency_status_text() {
            let block = fit_single_line_text(
                &text,
                TextStyle::new_px(
                    self.emergency_status_color(),
                    self.render_scale.apply_u32(15),
                )
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

    fn render_emergency_input_content(&self, buffer: &mut impl PixelBuffer, rect: Rect) {
        if self.shell.challenge_echo_on() && !self.shell.secret.is_empty() {
            let block = fit_sensitive_single_line_text(
                self.shell.secret.expose(),
                TextStyle::new_px(FOREGROUND, self.render_scale.apply_u32(16)).with_line_spacing(0),
                rect.width.saturating_sub(48) as u32,
            );
            block.draw(
                buffer,
                rect.x + scaled(22, self.render_scale),
                rect.y + (rect.height - block.height() as i32) / 2 - 1,
            );
            return;
        }
        if self.shell.displayed_secret_len() == 0 {
            let placeholder = fit_single_line_text(
                if matches!(self.shell.status, ShellStatus::Challenge { .. }) {
                    "Response"
                } else {
                    "Password"
                },
                TextStyle::new_px(MUTED, self.render_scale.apply_u32(16)).with_line_spacing(0),
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
            self.shell.displayed_secret_len(),
            self.shell.focused,
            self.emergency_mask_style(),
        );
    }

    fn emergency_input_style(&self) -> PillStyle {
        let border = if matches!(self.shell.status, ShellStatus::Rejected { .. }) {
            REJECTED
        } else if self.shell.focused {
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
        let scale = self.render_scale;
        style.bullet_size = scale.apply_i32(style.bullet_size);
        style.spacing = scale.apply_i32(style.spacing);
        style.horizontal_padding = scaled(22, self.render_scale);
        style
    }

    fn emergency_status_text(&self) -> Option<String> {
        if let Some(message) = self.shell.input_limit_message() {
            return Some(message);
        }
        match &self.shell.status {
            ShellStatus::Idle => None,
            ShellStatus::Challenge { text, .. } | ShellStatus::Notice { text } => {
                Some(text.clone())
            }
            ShellStatus::Pending { shown, .. } => shown.then(|| String::from("Checking...")),
            ShellStatus::Rejected {
                displayed_retry_seconds,
                message,
                ..
            } => match displayed_retry_seconds {
                Some(seconds) if *seconds > 0 => Some(format!("Try again in {seconds}s")),
                _ => message
                    .clone()
                    .or_else(|| Some(String::from("Authentication failed"))),
            },
        }
    }

    fn emergency_status_color(&self) -> ClearColor {
        if self.shell.input_limit_message().is_some() {
            return REJECTED;
        }
        match self.shell.status {
            ShellStatus::Pending { .. } => PENDING,
            ShellStatus::Challenge { .. } | ShellStatus::Notice { .. } => PENDING,
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
    fn new(size: FrameSize, scale: RenderScale) -> Self {
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

fn scaled(value: i32, scale: RenderScale) -> i32 {
    scale.apply_i32(value)
}

#[cfg(test)]
mod tests {
    use crate::shell::{ShellKey, ShellState};

    #[test]
    fn emergency_mode_reports_input_limit() {
        let mut shell = ShellState::default();
        shell.activate_emergency();
        for _ in 0..=super::super::MAX_SECRET_CHARACTERS {
            shell.handle_key(ShellKey::Character('x'));
        }

        assert_eq!(
            shell.render_context().emergency_status_text().as_deref(),
            Some("Maximum 128 characters")
        );
    }
}
