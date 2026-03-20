use kwylock_renderer::{
    ClearColor, ShadowStyle, SoftwareBuffer,
    masked::{MaskedInputStyle, draw_masked_input},
    panel::{PanelBodyMetrics, draw_panel_header, layout_panel_body, measure_panel_height},
    progress::{Progress, ProgressBarStyle, draw_progress_bar},
    shape::{Rect, draw_box},
    symbol::{SymbolKind, SymbolStyle, draw_symbol_with_shadow, measure_symbol},
    text::{TextBlock, TextStyle, fit_wrapped_text},
};

use super::{
    ShellState, ShellStatus,
    style::{TypographyScale, visual_style_for_surface},
};

const STATUS_ROW_MAX_GAP: u32 = 12;
const TEXT_SHADOW_COLOR: ClearColor = ClearColor::rgba(4, 6, 10, 160);

impl ShellState {
    pub fn render(&self, buffer: &mut SoftwareBuffer) {
        buffer.clear(self.theme.background);
        self.render_overlay(buffer);
    }

    pub fn render_overlay(&self, buffer: &mut SoftwareBuffer) {
        let size = buffer.size();
        let width = size.width as i32;
        let height = size.height as i32;
        let accent = self.accent_color();
        let visuals = visual_style_for_surface(&self.theme, width, accent, self.focused);
        let panel_width = visuals.panel_width;
        let panel_x = (width - panel_width) / 2;
        let provisional_panel_rect = Rect::new(panel_x, 0, panel_width, 0);
        let content_width = visuals
            .body_style
            .content_width(provisional_panel_rect)
            .max(180);
        let hint_block = fit_wrapped_text(
            &self.hint_text,
            TextStyle::new(self.theme.foreground, visuals.typography.hint_scale),
            content_width,
            visuals.typography.hint_min_scale,
        );
        let status_row = self.status_row(content_width, accent, visuals.typography);
        let panel_height = measure_panel_height(
            visuals.header_style,
            visuals.body_style,
            PanelBodyMetrics {
                hint_height: hint_block.height as i32,
                status_height: status_row.as_ref().map(|row| row.height as i32),
            },
        );
        let panel_y = (height - panel_height) / 2;
        let panel_rect = Rect::new(panel_x, panel_y, panel_width, panel_height);

        draw_box(buffer, panel_rect, visuals.panel_style);
        let header = draw_panel_header(buffer, panel_rect, visuals.header_style);
        let body = layout_panel_body(
            panel_rect,
            header,
            visuals.body_style,
            PanelBodyMetrics {
                hint_height: hint_block.height as i32,
                status_height: status_row.as_ref().map(|row| row.height as i32),
            },
        );

        draw_centered_block(buffer, panel_x, panel_width, body.hint_y, &hint_block);

        draw_box(buffer, body.input_rect, visuals.input_style);

        draw_progress_bar(
            buffer,
            body.progress_rect,
            indicator_progress(&self.status),
            ProgressBarStyle::new(
                self.theme.muted,
                if self.focused && matches!(self.status, ShellStatus::Idle) {
                    self.theme.focus
                } else {
                    accent
                },
            ),
        );

        draw_masked_input(
            buffer,
            body.input_rect,
            self.secret.chars().count(),
            self.focused,
            MaskedInputStyle::new(self.theme.foreground, self.theme.muted, accent),
        );

        if let (Some(status_row), Some(status_y)) = (status_row.as_ref(), body.status_y) {
            draw_centered_status_row(buffer, panel_x, panel_width, status_y, status_row);
        }
    }

    fn accent_color(&self) -> ClearColor {
        match &self.status {
            ShellStatus::Idle => {
                if self.focused {
                    self.theme.focus
                } else {
                    self.theme.input_border
                }
            }
            ShellStatus::Pending => self.theme.pending,
            ShellStatus::Rejected { .. } => self.theme.rejected,
        }
    }

    fn status_text(&self) -> Option<String> {
        match &self.status {
            ShellStatus::Idle => None,
            ShellStatus::Pending => Some(String::from("Checking password")),
            ShellStatus::Rejected {
                displayed_retry_seconds,
                ..
            } => match displayed_retry_seconds {
                Some(retry_seconds) if *retry_seconds > 0 => {
                    Some(format!("Authentication failed, retry in {retry_seconds}s"))
                }
                Some(_) | None => Some(String::from("Authentication failed")),
            },
        }
    }

    fn status_symbol(&self) -> Option<SymbolKind> {
        match self.status {
            ShellStatus::Idle => None,
            ShellStatus::Pending => Some(SymbolKind::Pending),
            ShellStatus::Rejected { .. } => Some(SymbolKind::Error),
        }
    }

    fn status_row(
        &self,
        max_width: u32,
        accent: ClearColor,
        typography: TypographyScale,
    ) -> Option<StatusRow> {
        let symbol = self.status_symbol()?;
        let text = self.status_text()?;
        let reserved_width = measure_symbol(SymbolStyle::new(accent, typography.status_scale)).0
            + STATUS_ROW_MAX_GAP;
        let text_width = max_width.saturating_sub(reserved_width).max(96);
        let text = fit_wrapped_text(
            &text,
            TextStyle::new(accent, typography.status_scale),
            text_width,
            typography.status_min_scale,
        );
        let symbol_style = SymbolStyle::new(accent, text.style.scale.max(2));
        let (symbol_width, symbol_height) = measure_symbol(symbol_style);
        let gap = status_row_gap(text.style.scale);
        let width = symbol_width + gap + text.width;
        let height = symbol_height.max(text.height);

        Some(StatusRow {
            symbol,
            symbol_style,
            text,
            gap,
            width,
            height,
        })
    }
}

struct StatusRow {
    symbol: SymbolKind,
    symbol_style: SymbolStyle,
    text: TextBlock,
    gap: u32,
    width: u32,
    height: u32,
}

fn draw_centered_block(
    buffer: &mut SoftwareBuffer,
    panel_x: i32,
    panel_width: i32,
    y: i32,
    block: &TextBlock,
) {
    let x = panel_x + ((panel_width - block.width as i32) / 2);
    block.draw_with_shadow(buffer, x, y, text_shadow(block.style.scale));
}

fn draw_centered_status_row(
    buffer: &mut SoftwareBuffer,
    panel_x: i32,
    panel_width: i32,
    y: i32,
    row: &StatusRow,
) {
    let x = panel_x + ((panel_width - row.width as i32) / 2);
    let (symbol_width, symbol_height) = measure_symbol(row.symbol_style);
    let symbol_y = y + ((row.height as i32 - symbol_height as i32) / 2);
    let text_x = x + symbol_width as i32 + row.gap as i32;
    let text_y = y + ((row.height as i32 - row.text.height as i32) / 2);

    draw_symbol_with_shadow(
        buffer,
        x,
        symbol_y,
        row.symbol,
        row.symbol_style,
        text_shadow(row.symbol_style.scale),
    );
    row.text
        .draw_with_shadow(buffer, text_x, text_y, text_shadow(row.text.style.scale));
}

fn status_row_gap(scale: u32) -> u32 {
    (scale.max(1) * 4).clamp(6, STATUS_ROW_MAX_GAP)
}

fn text_shadow(scale: u32) -> ShadowStyle {
    let offset = scale.max(1) as i32;
    ShadowStyle::new(TEXT_SHADOW_COLOR, offset, offset)
}

fn indicator_progress(status: &ShellStatus) -> Progress {
    match status {
        ShellStatus::Idle => Progress::new(1, 3),
        ShellStatus::Pending => Progress::new(1, 2),
        ShellStatus::Rejected {
            displayed_retry_seconds,
            ..
        } => {
            if displayed_retry_seconds.unwrap_or_default() > 0 {
                Progress::new(1, 1)
            } else {
                Progress::new(2, 3)
            }
        }
    }
}
