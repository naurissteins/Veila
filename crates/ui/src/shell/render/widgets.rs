use veila_common::{ClockStyle, WeatherAlignment};
use veila_renderer::{
    ClearColor, SoftwareBuffer,
    avatar::{AvatarAsset, AvatarStyle},
    cover::CoverArtAsset,
    icon::{AssetIcon, BatteryIcon, IconStyle, draw_icon, icon_visible_bounds},
    layer::{BackdropLayerMode, BackdropLayerShape, BackdropLayerStyle, draw_backdrop_layer},
    masked::{MaskedInputStyle, draw_masked_input},
    shape::{BorderStyle, PillStyle, Rect, draw_pill},
    text::TextBlock,
};

use crate::shell::render::styles::percent_to_alpha;

use super::model::{SceneClockBlocks, SceneWeatherBlocks};

const TOGGLE_HITBOX_SIZE: i32 = 28;
const TOGGLE_RIGHT_INSET: i32 = 14;
const CONTENT_GAP_TO_TOGGLE: i32 = 10;
pub(super) const NOW_PLAYING_CONTENT_GAP: i32 = 14;
pub(super) const NOW_PLAYING_TEXT_GAP: i32 = 8;
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
    pub reveal_secret: bool,
    pub toggle_hovered: bool,
    pub toggle_pressed: bool,
    pub show_toggle: bool,
    pub toggle_style: IconStyle,
    pub caps_lock_indicator: Option<TextBlock>,
}

pub(super) struct NowPlayingWidget<'a> {
    pub artwork: Option<&'a CoverArtAsset>,
    pub title: &'a TextBlock,
    pub artist: Option<&'a TextBlock>,
    pub background: Option<NowPlayingBackgroundStyle>,
    pub artwork_opacity: Option<u8>,
    pub artwork_size: i32,
    pub artwork_radius: i32,
    pub width: Option<i32>,
    pub content_gap: i32,
    pub text_gap: i32,
    pub right_padding: i32,
    pub bottom_padding: i32,
    pub right_offset: i32,
    pub bottom_offset: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NowPlayingBackgroundStyle {
    pub mode: BackdropLayerMode,
    pub color: ClearColor,
    pub blur_radius: u8,
    pub radius: i32,
    pub padding_x: i32,
    pub padding_y: i32,
    pub border_color: Option<ClearColor>,
    pub border_width: i32,
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

pub(super) fn draw_top_right_block(
    buffer: &mut SoftwareBuffer,
    right_padding: i32,
    right_offset: i32,
    y: i32,
    background: ClearColor,
    background_size: Option<i32>,
    block: &TextBlock,
) {
    let chip_diameter =
        top_right_chip_diameter(background_size, block.width as i32, block.height as i32);
    let max_x = (buffer.size().width as i32 - chip_diameter).max(0);
    let x =
        (buffer.size().width as i32 - right_padding - chip_diameter + right_offset).clamp(0, max_x);
    let y = y.max(0);

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
pub(super) fn draw_top_right_icon_chip(
    buffer: &mut SoftwareBuffer,
    right_padding: i32,
    right_offset: i32,
    y: i32,
    background: ClearColor,
    background_size: Option<i32>,
    icon: BatteryIcon,
    icon_style: IconStyle,
    icon_size: i32,
) {
    let chip_diameter = top_right_chip_diameter(background_size, icon_size, icon_size);
    let max_x = (buffer.size().width as i32 - chip_diameter).max(0);
    let x =
        (buffer.size().width as i32 - right_padding - chip_diameter + right_offset).clamp(0, max_x);
    let y = y.max(8);

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
    let toggle_rect = widget.show_toggle.then(|| input_toggle_hitbox(widget.rect));
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

    if let Some(toggle_rect) = toggle_rect {
        draw_toggle_icon(
            buffer,
            toggle_rect,
            widget.reveal_secret,
            widget.toggle_hovered,
            widget.toggle_pressed,
            widget.toggle_style,
        );
    }

    if let Some(caps_lock_indicator) = widget.caps_lock_indicator.as_ref() {
        let x = widget.rect.x + widget.rect.width - caps_lock_indicator.width as i32;
        let y = (widget.rect.y - caps_lock_indicator.height as i32 - 6).max(0);
        caps_lock_indicator.draw(buffer, x, y);
    }
}

pub(super) fn draw_weather_widget(
    buffer: &mut SoftwareBuffer,
    top_y: i32,
    weather: &SceneWeatherBlocks,
) {
    let icon_size = weather.icon_size;
    let widget_width = icon_size
        .max(weather.temperature.width as i32)
        .max(weather.location.width as i32);
    let origin_x = match weather.alignment {
        WeatherAlignment::Left => weather.horizontal_padding + weather.left_offset,
        WeatherAlignment::Right => {
            buffer.size().width as i32 - weather.horizontal_padding - widget_width
                + weather.left_offset
        }
    };
    let icon_y = top_y + weather.bottom_offset;
    let icon_x = align_weather_line_x(origin_x, widget_width, icon_size, weather.alignment);
    let temperature_x = align_weather_line_x(
        origin_x,
        widget_width,
        weather.temperature.width as i32,
        weather.alignment,
    );
    let location_x = align_weather_line_x(
        origin_x,
        widget_width,
        weather.location.width as i32,
        weather.alignment,
    );
    let icon_rect = Rect::new(icon_x, icon_y, icon_size, icon_size);
    let icon_style = IconStyle::new(
        veila_renderer::ClearColor::opaque(255, 255, 255).with_alpha(
            weather
                .icon_opacity
                .map(percent_to_alpha)
                .unwrap_or(u8::MAX),
        ),
    )
    .with_padding(0);
    let text_y = icon_visible_bounds(icon_rect, AssetIcon::Weather(weather.icon), icon_style)
        .map(|bounds| bounds.y + bounds.height)
        .unwrap_or(icon_y + icon_size)
        + weather.icon_gap;

    draw_icon(
        buffer,
        icon_rect,
        AssetIcon::Weather(weather.icon),
        icon_style,
    );
    weather.temperature.draw(buffer, temperature_x, text_y);
    weather.location.draw(
        buffer,
        location_x,
        text_y + weather.temperature.height as i32 + weather.location_gap,
    );
}

pub(super) fn draw_now_playing_widget(buffer: &mut SoftwareBuffer, widget: NowPlayingWidget<'_>) {
    let has_artwork = widget.artwork.is_some();
    let artwork_size = widget.artwork_size.max(0);
    let artwork_width = if has_artwork { artwork_size } else { 0 };
    let content_gap = if has_artwork {
        widget.content_gap.max(0)
    } else {
        0
    };
    let text_gap = widget.text_gap.max(0);
    let text_width = widget.artist.map_or(widget.title.width as i32, |artist| {
        (widget.title.width as i32).max(artist.width as i32)
    });
    let text_height = widget.artist.map_or(widget.title.height as i32, |artist| {
        widget.title.height as i32 + text_gap + artist.height as i32
    });
    let content_width = artwork_width + content_gap + text_width;
    let widget_width = widget
        .width
        .map(|width| width.max(content_width))
        .unwrap_or(content_width);
    let widget_height = artwork_size.max(text_height);
    let origin_x = (buffer.size().width as i32 - widget.right_padding - widget_width
        + widget.right_offset)
        .max(0);
    let origin_y = (buffer.size().height as i32 - widget.bottom_padding - widget_height
        + widget.bottom_offset)
        .max(0);
    let content_x = origin_x + (widget_width - content_width);
    let artwork_y = origin_y + (widget_height - artwork_size) / 2;
    let text_x = content_x + artwork_width + content_gap;
    let text_y = origin_y + (widget_height - text_height) / 2;

    if let Some(background) = widget.background {
        draw_now_playing_background(
            buffer,
            Rect::new(content_x, origin_y, content_width, widget_height),
            background,
        );
    }

    if let Some(artwork) = widget.artwork {
        artwork.draw(
            buffer,
            content_x,
            artwork_y,
            artwork_size as u32,
            artwork_size as u32,
            widget.artwork_radius,
            widget.artwork_opacity,
        );
    }

    if let Some(artist) = widget.artist {
        artist.draw(buffer, text_x, text_y);
        widget
            .title
            .draw(buffer, text_x, text_y + artist.height as i32 + text_gap);
    } else {
        widget.title.draw(buffer, text_x, text_y);
    }
}

fn draw_now_playing_background(
    buffer: &mut SoftwareBuffer,
    content_rect: Rect,
    background: NowPlayingBackgroundStyle,
) {
    let padding_x = background.padding_x.max(0);
    let padding_y = background.padding_y.max(0);
    let rect = Rect::new(
        content_rect.x - padding_x,
        content_rect.y - padding_y,
        content_rect.width + padding_x * 2,
        content_rect.height + padding_y * 2,
    );

    draw_backdrop_layer(
        buffer,
        rect,
        BackdropLayerStyle::new(
            background.mode,
            BackdropLayerShape::Panel,
            background.color,
            background.blur_radius,
            background.radius,
            background.border_color,
            background.border_width,
        ),
    );
}

fn align_weather_line_x(
    origin_x: i32,
    widget_width: i32,
    content_width: i32,
    alignment: WeatherAlignment,
) -> i32 {
    match alignment {
        WeatherAlignment::Left => origin_x,
        WeatherAlignment::Right => origin_x + widget_width - content_width,
    }
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
