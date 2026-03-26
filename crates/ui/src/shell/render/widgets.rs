use veila_renderer::{
    SoftwareBuffer,
    avatar::{AvatarAsset, AvatarStyle},
    icon::{AssetIcon, IconStyle, draw_icon},
    masked::{MaskedInputStyle, draw_masked_input},
    shape::{BorderStyle, PillStyle, Rect, draw_pill},
    text::TextBlock,
};

const TOGGLE_HITBOX_SIZE: i32 = 28;
const TOGGLE_RIGHT_INSET: i32 = 14;
const CONTENT_GAP_TO_TOGGLE: i32 = 10;

pub(super) struct InputWidget {
    pub rect: Rect,
    pub secret_len: usize,
    pub focused: bool,
    pub shell_style: PillStyle,
    pub mask_style: MaskedInputStyle,
    pub placeholder: Option<TextBlock>,
    pub revealed_secret: Option<TextBlock>,
    pub reveal_secret: bool,
    pub toggle_hovered: bool,
    pub toggle_pressed: bool,
    pub toggle_style: IconStyle,
}

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

pub(super) fn draw_input_shell(buffer: &mut SoftwareBuffer, rect: Rect, style: PillStyle) {
    draw_pill(buffer, rect, style);
}

pub(super) fn draw_input_content(buffer: &mut SoftwareBuffer, widget: &InputWidget) {
    let toggle_rect = input_toggle_hitbox(widget.rect);
    let content_rect = input_content_rect(widget.rect, toggle_rect);

    if widget.secret_len == 0
        && let Some(placeholder) = widget.placeholder.as_ref()
    {
        let x = content_rect.x + widget.mask_style.horizontal_padding.saturating_sub(4);
        let y = content_rect.y + (content_rect.height - placeholder.height as i32) / 2 - 1;
        placeholder.draw(buffer, x, y);
    }

    if let Some(revealed_secret) = widget.revealed_secret.as_ref() {
        let x = content_rect.x + widget.mask_style.horizontal_padding.saturating_sub(4);
        let y = content_rect.y + (content_rect.height - revealed_secret.height as i32) / 2 - 1;
        revealed_secret.draw(buffer, x, y);
    } else {
        draw_masked_input(
            buffer,
            content_rect,
            widget.secret_len,
            widget.focused,
            widget.mask_style,
        );
    }

    draw_toggle_icon(
        buffer,
        toggle_rect,
        widget.reveal_secret,
        widget.toggle_hovered,
        widget.toggle_pressed,
        widget.toggle_style,
    );
}

pub(super) fn input_toggle_hitbox(rect: Rect) -> Rect {
    let size = TOGGLE_HITBOX_SIZE
        .min(rect.height.saturating_sub(8))
        .max(18);
    Rect::new(
        rect.x + rect.width - size - TOGGLE_RIGHT_INSET,
        rect.y + (rect.height - size) / 2,
        size,
        size,
    )
}

fn input_content_rect(rect: Rect, toggle_rect: Rect) -> Rect {
    let right_edge = toggle_rect.x - CONTENT_GAP_TO_TOGGLE;
    Rect::new(rect.x, rect.y, (right_edge - rect.x).max(0), rect.height)
}

fn draw_toggle_icon(
    buffer: &mut SoftwareBuffer,
    hitbox: Rect,
    reveal_secret: bool,
    hovered: bool,
    pressed: bool,
    style: IconStyle,
) {
    if hovered || pressed {
        let alpha = if pressed { 62 } else { 34 };
        let border_alpha = if pressed { 104 } else { 68 };
        draw_pill(
            buffer,
            hitbox,
            PillStyle::new(style.color.with_alpha(alpha))
                .with_radius(hitbox.width / 2)
                .with_border(BorderStyle::new(style.color.with_alpha(border_alpha), 1)),
        );
    }

    let icon = if reveal_secret {
        AssetIcon::EyeOff
    } else {
        AssetIcon::Eye
    };
    draw_icon(buffer, hitbox, icon, style);
}
