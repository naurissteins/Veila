use veila_renderer::{
    ClearColor, ShadowStyle, SoftwareBuffer,
    avatar::{AvatarAsset, AvatarStyle},
    masked::{MaskedInputStyle, draw_masked_input},
    shape::{PillStyle, Rect, draw_pill},
    text::TextBlock,
};

use super::super::ShellStatus;

const TEXT_SHADOW_COLOR: ClearColor = ClearColor::rgba(6, 8, 12, 140);

pub(super) fn draw_centered_block(
    buffer: &mut SoftwareBuffer,
    center_x: i32,
    y: i32,
    block: &TextBlock,
) {
    let x = center_x - block.width as i32 / 2;
    block.draw_with_shadow(buffer, x, y, text_shadow(block.style.scale));
}

pub(super) fn draw_avatar_widget(
    buffer: &mut SoftwareBuffer,
    avatar: &AvatarAsset,
    center_x: i32,
    top_y: i32,
    size: u32,
    style: AvatarStyle,
) {
    avatar.draw(buffer, center_x, top_y, size, style);
}

pub(super) fn draw_input_widget(
    buffer: &mut SoftwareBuffer,
    rect: Rect,
    secret_len: usize,
    focused: bool,
    shell_style: PillStyle,
    mask_style: MaskedInputStyle,
) {
    draw_pill(buffer, rect, shell_style);
    draw_masked_input(buffer, rect, secret_len, focused, mask_style);
}

pub(super) fn draw_indicator_widget(
    buffer: &mut SoftwareBuffer,
    center_x: i32,
    y: i32,
    input_width: i32,
    status: &ShellStatus,
    accent: ClearColor,
) {
    let Some(rect) = indicator_rect(center_x, y, input_width, status) else {
        return;
    };

    draw_pill(
        buffer,
        rect,
        PillStyle::new(indicator_rect_color(status, accent)),
    );
}

fn indicator_rect(center_x: i32, y: i32, input_width: i32, status: &ShellStatus) -> Option<Rect> {
    let width = match status {
        ShellStatus::Idle => input_width / 8,
        ShellStatus::Pending => input_width / 3,
        ShellStatus::Rejected {
            displayed_retry_seconds,
            ..
        } => {
            if displayed_retry_seconds.unwrap_or_default() > 0 {
                input_width / 5
            } else {
                input_width / 4
            }
        }
    };

    Some(Rect::new(
        center_x - width / 2,
        y,
        width.max(18),
        indicator_height(status),
    ))
}

fn indicator_rect_color(status: &ShellStatus, accent: ClearColor) -> ClearColor {
    match status {
        ShellStatus::Idle => accent.with_alpha(92),
        ShellStatus::Pending => accent.with_alpha(220),
        ShellStatus::Rejected {
            displayed_retry_seconds,
            ..
        } => {
            if displayed_retry_seconds.unwrap_or_default() > 0 {
                accent.with_alpha(180)
            } else {
                accent.with_alpha(220)
            }
        }
    }
}

pub(super) fn indicator_height(status: &ShellStatus) -> i32 {
    match status {
        ShellStatus::Idle => 4,
        ShellStatus::Pending => 5,
        ShellStatus::Rejected { .. } => 5,
    }
}

fn text_shadow(scale: u32) -> ShadowStyle {
    let offset = scale.max(1) as i32;
    ShadowStyle::new(TEXT_SHADOW_COLOR, 0, offset)
}
