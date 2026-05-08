use std::f32::consts::{FRAC_PI_2, TAU};

use veila_common::ClockStyle;
use veila_renderer::{
    ClearColor, SoftwareBuffer,
    avatar::{AvatarAsset, AvatarStyle},
    icon::{AssetIcon, BatteryIcon, IconStyle, draw_icon},
    masked::{MaskedInputStyle, draw_masked_input},
    shape::{BorderStyle, PillStyle, Rect, draw_pill},
    text::TextBlock,
};

use crate::shell::render::styles::percent_to_alpha;

use super::model::SceneClockBlocks;

const TOGGLE_HITBOX_SIZE: i32 = 28;
const TOGGLE_RIGHT_INSET: i32 = 14;
const CONTENT_GAP_TO_TOGGLE: i32 = 10;
const CHIP_HORIZONTAL_PADDING: i32 = 10;
const CHIP_VERTICAL_PADDING: i32 = 8;

pub(super) struct InputWidget {
    pub rect: Rect,
    pub secret_len: usize,
    pub focused: bool,
    pub shell_style: PillStyle,
    pub mask_style: MaskedInputStyle,
    pub placeholder: Option<TextBlock>,
    pub revealed_secret: Option<TextBlock>,
    pub inline_status: Option<TextBlock>,
    pub right_adornment: InputRightAdornment,
}

pub(super) enum InputRightAdornment {
    None,
    Toggle {
        hovered: bool,
        pressed: bool,
        reveal_secret: bool,
        style: IconStyle,
    },
    Spinner {
        phase: u8,
        style: IconStyle,
    },
    CapsLock {
        style: IconStyle,
    },
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

pub(super) fn draw_block(buffer: &mut SoftwareBuffer, x: i32, y: i32, block: &TextBlock) {
    block.draw(buffer, x, y);
}

pub(super) fn draw_clock_widget(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    clock: &SceneClockBlocks,
) {
    let base_width = match clock.style {
        ClockStyle::Standard => clock.primary.width as i32,
        ClockStyle::Stacked => (clock.primary.width as i32).max(
            clock
                .secondary
                .as_ref()
                .map_or(0, |secondary| secondary.width as i32),
        ),
    };

    match clock.style {
        ClockStyle::Standard => {
            clock.primary.draw(buffer, x, y);
        }
        ClockStyle::Stacked => {
            let primary_x = x + (base_width - clock.primary.width as i32) / 2;
            clock.primary.draw(buffer, primary_x, y);

            if let Some(secondary) = clock.secondary.as_ref() {
                let secondary_x = x + (base_width - secondary.width as i32) / 2;
                secondary.draw(
                    buffer,
                    secondary_x,
                    y + clock.primary.height as i32 + SceneClockBlocks::stacked_gap(),
                );
            }
        }
    }

    if let Some(meridiem) = clock.meridiem.as_ref() {
        meridiem.draw(
            buffer,
            x + base_width + SceneClockBlocks::meridiem_gap() + clock.meridiem_offset_x,
            y + SceneClockBlocks::meridiem_top_offset() + clock.meridiem_offset_y,
        );
    }
}

pub(super) fn draw_chip_block(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    background: ClearColor,
    background_size: Option<i32>,
    block: &TextBlock,
) {
    let chip_diameter =
        top_right_chip_diameter(background_size, block.width as i32, block.height as i32);
    let max_x = (buffer.size().width as i32 - chip_diameter).max(0);
    let max_y = (buffer.size().height as i32 - chip_diameter).max(0);
    let x = x.clamp(0, max_x);
    let y = y.clamp(0, max_y);

    draw_pill(
        buffer,
        Rect::new(x, y, chip_diameter, chip_diameter),
        PillStyle::new(background).with_radius(chip_diameter / 2),
    );
    block.draw(
        buffer,
        x + (chip_diameter - block.width as i32) / 2,
        y + (chip_diameter - block.height as i32) / 2,
    );
}

pub(super) fn top_right_chip_diameter(
    background_size: Option<i32>,
    content_width: i32,
    content_height: i32,
) -> i32 {
    background_size
        .unwrap_or_else(|| {
            (content_width + CHIP_HORIZONTAL_PADDING * 2)
                .max(content_height + CHIP_VERTICAL_PADDING * 2)
        })
        .clamp(20, 160)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_icon_chip(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    background: ClearColor,
    background_size: Option<i32>,
    icon: BatteryIcon,
    icon_style: IconStyle,
    icon_size: i32,
) {
    let chip_diameter = top_right_chip_diameter(background_size, icon_size, icon_size);
    let max_x = (buffer.size().width as i32 - chip_diameter).max(0);
    let max_y = (buffer.size().height as i32 - chip_diameter).max(0);
    let x = x.clamp(0, max_x);
    let y = y.clamp(0, max_y);

    draw_pill(
        buffer,
        Rect::new(x, y, chip_diameter, chip_diameter),
        PillStyle::new(background).with_radius(chip_diameter / 2),
    );

    let icon_extent = icon_size.clamp(12, chip_diameter.saturating_sub(8));
    let icon_x = x + (chip_diameter - icon_extent) / 2;
    let icon_y = y + (chip_diameter - icon_extent) / 2;
    draw_icon(
        buffer,
        Rect::new(icon_x, icon_y, icon_extent, icon_extent),
        AssetIcon::Battery(icon),
        icon_style.with_padding(0),
    );
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
    let adornment_rect = (!matches!(widget.right_adornment, InputRightAdornment::None))
        .then(|| input_toggle_hitbox(widget.rect));
    let content_rect = input_content_rect(widget.rect, adornment_rect);

    if let Some(inline_status) = widget.inline_status.as_ref() {
        let x = content_rect.x + widget.mask_style.horizontal_padding.saturating_sub(4);
        let y = content_rect.y + (content_rect.height - inline_status.height as i32) / 2 - 1;
        inline_status.draw(buffer, x, y);
    } else if let Some(revealed_secret) = widget.revealed_secret.as_ref() {
        let x = content_rect.x + widget.mask_style.horizontal_padding.saturating_sub(4);
        let y = content_rect.y + (content_rect.height - revealed_secret.height as i32) / 2 - 1;
        revealed_secret.draw(buffer, x, y);
    } else {
        if widget.secret_len == 0
            && let Some(placeholder) = widget.placeholder.as_ref()
        {
            let x = content_rect.x + widget.mask_style.horizontal_padding.saturating_sub(4);
            let y = content_rect.y + (content_rect.height - placeholder.height as i32) / 2 - 1;
            placeholder.draw(buffer, x, y);
        }
        draw_masked_input(
            buffer,
            content_rect,
            widget.secret_len,
            widget.focused,
            widget.mask_style,
        );
    }

    if let Some(adornment_rect) = adornment_rect {
        draw_input_right_adornment(buffer, adornment_rect, &widget.right_adornment);
    }
}

pub(super) fn draw_weather_icon(
    buffer: &mut SoftwareBuffer,
    x: i32,
    y: i32,
    icon: veila_renderer::icon::WeatherIcon,
    icon_size: i32,
    opacity: Option<u8>,
) {
    let icon_rect = Rect::new(x, y, icon_size, icon_size);
    let icon_style = IconStyle::new(
        veila_renderer::ClearColor::opaque(255, 255, 255)
            .with_alpha(opacity.map(percent_to_alpha).unwrap_or(u8::MAX)),
    )
    .with_padding(0);
    draw_icon(buffer, icon_rect, AssetIcon::Weather(icon), icon_style);
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

fn input_content_rect(rect: Rect, toggle_rect: Option<Rect>) -> Rect {
    let right_edge = toggle_rect
        .map(|toggle_rect| toggle_rect.x - CONTENT_GAP_TO_TOGGLE)
        .unwrap_or(rect.x + rect.width);
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

fn draw_input_right_adornment(
    buffer: &mut SoftwareBuffer,
    hitbox: Rect,
    adornment: &InputRightAdornment,
) {
    match adornment {
        InputRightAdornment::None => {}
        InputRightAdornment::Toggle {
            hovered,
            pressed,
            reveal_secret,
            style,
        } => draw_toggle_icon(buffer, hitbox, *reveal_secret, *hovered, *pressed, *style),
        InputRightAdornment::Spinner { phase, style } => {
            draw_spinner_icon(buffer, hitbox, *phase, *style)
        }
        InputRightAdornment::CapsLock { style } => {
            draw_icon(buffer, hitbox, AssetIcon::CapsLock, *style)
        }
    }
}

fn draw_spinner_icon(buffer: &mut SoftwareBuffer, hitbox: Rect, phase: u8, style: IconStyle) {
    const SEGMENT_ALPHAS: [u8; 8] = [255, 212, 176, 144, 112, 82, 56, 34];

    let size = hitbox.width.min(hitbox.height).max(12) as f32;
    let center_x = hitbox.x as f32 + hitbox.width as f32 / 2.0;
    let center_y = hitbox.y as f32 + hitbox.height as f32 / 2.0;
    let orbit_radius = (size * 0.3).max(4.0);
    let dot_diameter = ((size * 0.18).round() as i32).clamp(2, 5);
    let dot_radius = dot_diameter / 2;

    for position in 0..8 {
        let angle = (position as f32 / 8.0) * TAU - FRAC_PI_2;
        let x = center_x + orbit_radius * angle.cos();
        let y = center_y + orbit_radius * angle.sin();
        let alpha = scaled_alpha(
            style.color.alpha,
            SEGMENT_ALPHAS[(position + 8 - usize::from(phase % 8)) % 8],
        );
        let color = style.color.with_alpha(alpha);
        draw_pill(
            buffer,
            Rect::new(
                x.round() as i32 - dot_radius,
                y.round() as i32 - dot_radius,
                dot_diameter,
                dot_diameter,
            ),
            PillStyle::new(color).with_radius(dot_radius.max(1)),
        );
    }
}

fn scaled_alpha(base_alpha: u8, multiplier: u8) -> u8 {
    ((u16::from(base_alpha) * u16::from(multiplier) + 127) / 255) as u8
}
