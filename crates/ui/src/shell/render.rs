use kwylock_renderer::{
    ClearColor, ShadowStyle, SoftwareBuffer,
    symbol::{SymbolKind, SymbolStyle, draw_symbol_with_shadow, measure_symbol},
    text::{TextBlock, TextStyle, fit_wrapped_text},
};

use super::{ShellState, ShellStatus};

const STATUS_ROW_MAX_GAP: u32 = 12;
const TEXT_SHADOW_COLOR: ClearColor = ClearColor::opaque(8, 10, 14);

impl ShellState {
    pub fn render(&self, buffer: &mut SoftwareBuffer) {
        buffer.clear(self.theme.background);
        self.render_overlay(buffer);
    }

    pub fn render_overlay(&self, buffer: &mut SoftwareBuffer) {
        let size = buffer.size();
        let width = size.width as i32;
        let height = size.height as i32;
        let panel_width = ((width * 3) / 5).clamp(320, 620);
        let content_width = (panel_width - 64).max(160) as u32;
        let panel_x = (width - panel_width) / 2;
        let accent = self.accent_color();
        let hint_block = fit_wrapped_text(
            &self.hint_text,
            TextStyle::new(self.theme.foreground, 2),
            content_width,
            1,
        );
        let status_row = self.status_row(content_width, accent);
        let panel_height = compute_panel_height(&hint_block, status_row.as_ref());
        let panel_y = (height - panel_height) / 2;

        fill_rect(
            buffer,
            panel_x,
            panel_y,
            panel_width,
            panel_height,
            self.theme.panel,
        );
        stroke_rect(
            buffer,
            panel_x,
            panel_y,
            panel_width,
            panel_height,
            2,
            self.theme.panel_border,
        );
        fill_rect(buffer, panel_x, panel_y, panel_width, 6, accent);

        let hint_y = panel_y + 34;
        draw_centered_block(buffer, panel_x, panel_width, hint_y, &hint_block);

        let input_x = panel_x + 32;
        let input_y = hint_y + hint_block.height as i32 + 22;
        let input_width = panel_width - 64;
        let input_height = 38;

        fill_rect(
            buffer,
            input_x,
            input_y,
            input_width,
            input_height,
            self.theme.input,
        );
        stroke_rect(
            buffer,
            input_x,
            input_y,
            input_width,
            input_height,
            2,
            if self.focused {
                accent
            } else {
                self.theme.input_border
            },
        );

        let indicator_y = input_y + input_height + 24;
        fill_rect(
            buffer,
            panel_x + 32,
            indicator_y,
            panel_width - 64,
            6,
            self.theme.muted,
        );
        fill_rect(
            buffer,
            panel_x + 32,
            indicator_y,
            indicator_width(panel_width, &self.status),
            6,
            accent,
        );

        self.draw_secret(buffer, input_x, input_y, input_width, input_height, accent);

        if let Some(status_row) = status_row.as_ref() {
            draw_centered_status_row(buffer, panel_x, panel_width, indicator_y + 22, status_row);
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

    fn status_row(&self, max_width: u32, accent: ClearColor) -> Option<StatusRow> {
        let symbol = self.status_symbol()?;
        let text = self.status_text()?;
        let reserved_width = measure_symbol(SymbolStyle::new(accent, 2)).0 + STATUS_ROW_MAX_GAP;
        let text_width = max_width.saturating_sub(reserved_width).max(96);
        let text = fit_wrapped_text(&text, TextStyle::new(accent, 2), text_width, 1);
        let symbol_style = SymbolStyle::new(accent, text.style.scale);
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

    fn draw_secret(
        &self,
        buffer: &mut SoftwareBuffer,
        input_x: i32,
        input_y: i32,
        input_width: i32,
        input_height: i32,
        accent: ClearColor,
    ) {
        if self.secret.is_empty() {
            fill_rect(
                buffer,
                input_x + 20,
                input_y + (input_height / 2) - 2,
                input_width / 3,
                4,
                self.theme.muted,
            );

            if self.focused {
                fill_rect(
                    buffer,
                    input_x + 20,
                    input_y + 9,
                    3,
                    input_height - 18,
                    accent,
                );
            }

            return;
        }

        let bullet_size = 10;
        let spacing = 16;
        let visible = ((input_width - 40) / spacing).max(1) as usize;
        let bullet_count = self.secret.chars().count().min(visible);
        let row_width = (bullet_count as i32 * bullet_size)
            + ((bullet_count.saturating_sub(1)) as i32 * (spacing - bullet_size));
        let start_x = input_x + ((input_width - row_width) / 2).max(18);
        let bullet_y = input_y + (input_height - bullet_size) / 2;

        for index in 0..bullet_count {
            let x = start_x + index as i32 * spacing;
            fill_rect(
                buffer,
                x,
                bullet_y,
                bullet_size,
                bullet_size,
                self.theme.foreground,
            );
        }

        if self.focused {
            let cursor_x = start_x + bullet_count as i32 * spacing + 4;
            fill_rect(buffer, cursor_x, input_y + 8, 3, input_height - 16, accent);
        }
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

fn compute_panel_height(hint_block: &TextBlock, status_row: Option<&StatusRow>) -> i32 {
    let status_height = status_row.map(|row| row.height as i32 + 22).unwrap_or(0);
    34 + hint_block.height as i32 + 22 + 38 + 24 + 6 + status_height + 28
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

fn indicator_width(panel_width: i32, status: &ShellStatus) -> i32 {
    match status {
        ShellStatus::Idle => (panel_width - 64) / 3,
        ShellStatus::Pending => (panel_width - 64) / 2,
        ShellStatus::Rejected {
            displayed_retry_seconds,
            ..
        } => {
            if displayed_retry_seconds.unwrap_or_default() > 0 {
                panel_width - 64
            } else {
                ((panel_width - 64) * 2) / 3
            }
        }
    }
}

fn fill_rect(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: ClearColor,
) {
    let size = buffer.size();
    let right = (x + width).clamp(0, size.width as i32);
    let bottom = (y + height).clamp(0, size.height as i32);
    let left = x.clamp(0, size.width as i32);
    let top = y.clamp(0, size.height as i32);

    if left >= right || top >= bottom {
        return;
    }

    let stride = size.width as usize * 4;
    let pixel = color.to_argb8888_bytes();
    let pixels = buffer.pixels_mut();

    for row in top as usize..bottom as usize {
        let row_start = row * stride;
        for column in left as usize..right as usize {
            let offset = row_start + column * 4;
            pixels[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
}

fn stroke_rect(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    thickness: i32,
    color: ClearColor,
) {
    fill_rect(buffer, x, y, width, thickness, color);
    fill_rect(buffer, x, y + height - thickness, width, thickness, color);
    fill_rect(buffer, x, y, thickness, height, color);
    fill_rect(buffer, x + width - thickness, y, thickness, height, color);
}
