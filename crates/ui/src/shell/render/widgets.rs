use veila_renderer::{
    ClearColor, ShadowStyle, SoftwareBuffer,
    avatar::{AvatarAsset, AvatarStyle},
    masked::{MaskedInputStyle, draw_masked_input},
    shape::{PillStyle, Rect, draw_pill},
    text::TextBlock,
};

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

fn text_shadow(scale: u32) -> ShadowStyle {
    let offset = scale.max(1) as i32;
    ShadowStyle::new(TEXT_SHADOW_COLOR, 0, offset)
}
