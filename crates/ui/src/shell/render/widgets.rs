use veila_renderer::{
    SoftwareBuffer,
    avatar::{AvatarAsset, AvatarStyle},
    masked::{MaskedInputStyle, draw_masked_input},
    shape::{PillStyle, Rect, draw_pill},
    text::TextBlock,
};

pub(super) fn draw_centered_block(
    buffer: &mut SoftwareBuffer,
    center_x: i32,
    y: i32,
    block: &TextBlock,
) {
    let x = center_x - block.width as i32 / 2;
    block.draw(buffer, x, y);
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
    placeholder: Option<&TextBlock>,
) {
    draw_pill(buffer, rect, shell_style);
    if secret_len == 0
        && let Some(placeholder) = placeholder
    {
        let x = rect.x + mask_style.horizontal_padding.saturating_sub(4);
        let y = rect.y + (rect.height - placeholder.height as i32) / 2 - 1;
        placeholder.draw(buffer, x, y);
    }
    draw_masked_input(buffer, rect, secret_len, focused, mask_style);
}
