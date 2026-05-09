mod color;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use veila_common::{
    AppConfig, BackdropMode, ClockAlignment, ClockFormat, ClockStyle, FontStyle, GridVisualConfig,
    HorizontalAlign, InputRevealMode, StatusDisplayMode, VerticalAlign, WidgetPositionConfig,
};
use veila_renderer::ClearColor;

use self::color::to_color;
use super::PreviewGrid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetPosition {
    pub halign: HorizontalAlign,
    pub valign: VerticalAlign,
    pub x: i32,
    pub y: i32,
    pub target: WidgetPositionTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetPositionTarget {
    Screen,
    Backdrop(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backdrop {
    pub mode: BackdropMode,
    pub color: ClearColor,
    pub blur_strength: u8,
    pub radius: i32,
    pub border_color: Option<ClearColor>,
    pub border_width: i32,
    pub full_width: bool,
    pub full_height: bool,
    pub width: i32,
    pub height: i32,
    pub position: WidgetPosition,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellTheme {
    pub background: ClearColor,
    pub avatar_enabled: bool,
    pub avatar_background: ClearColor,
    pub input: ClearColor,
    pub input_border: ClearColor,
    pub input_font_family: Option<String>,
    pub input_font_weight: Option<u16>,
    pub input_font_style: Option<FontStyle>,
    pub input_font_size: Option<u32>,
    pub input_reveal_on_interaction: bool,
    pub input_reveal_mode: InputRevealMode,
    pub input_reveal_hint: String,
    pub reveal_enabled: bool,
    pub reveal_color: Option<ClearColor>,
    pub reveal_font_family: Option<String>,
    pub reveal_font_weight: Option<u16>,
    pub reveal_font_style: Option<FontStyle>,
    pub reveal_font_size: Option<u32>,
    pub input_position: Option<WidgetPosition>,
    pub input_width: Option<i32>,
    pub input_height: Option<i32>,
    pub input_radius: i32,
    pub input_border_width: Option<i32>,
    pub avatar_size: Option<i32>,
    pub avatar_offset_y: Option<i32>,
    pub avatar_position: Option<WidgetPosition>,
    pub avatar_placeholder_padding: Option<i32>,
    pub avatar_icon_color: Option<ClearColor>,
    pub avatar_ring_color: Option<ClearColor>,
    pub avatar_ring_width: Option<i32>,
    pub username_enabled: bool,
    pub username_font_family: Option<String>,
    pub username_font_weight: Option<u16>,
    pub username_font_style: Option<FontStyle>,
    pub username_color: Option<ClearColor>,
    pub username_font_size: Option<u32>,
    pub username_offset_y: Option<i32>,
    pub username_position: Option<WidgetPosition>,
    pub avatar_gap: Option<i32>,
    pub username_gap: Option<i32>,
    pub status_position: Option<WidgetPosition>,
    pub status_mode: StatusDisplayMode,
    pub clock_gap: Option<i32>,
    pub clock_enabled: bool,
    pub clock_alignment: ClockAlignment,
    pub clock_center_in_layer: bool,
    pub clock_offset_x: Option<i32>,
    pub clock_offset_y: Option<i32>,
    pub clock_position: Option<WidgetPosition>,
    pub clock_font_family: Option<String>,
    pub clock_font_weight: Option<u16>,
    pub clock_font_style: Option<FontStyle>,
    pub clock_style: ClockStyle,
    pub clock_format: ClockFormat,
    pub clock_meridiem_font_size: Option<u32>,
    pub clock_meridiem_x: Option<i32>,
    pub clock_meridiem_y: Option<i32>,
    pub clock_color: Option<ClearColor>,
    pub date_enabled: bool,
    pub date_font_family: Option<String>,
    pub date_font_weight: Option<u16>,
    pub date_font_style: Option<FontStyle>,
    pub date_color: Option<ClearColor>,
    pub date_position: Option<WidgetPosition>,
    pub clock_font_size: Option<u32>,
    pub date_font_size: Option<u32>,
    pub placeholder_enabled: bool,
    pub placeholder_color: Option<ClearColor>,
    pub eye_enabled: bool,
    pub eye_icon_color: Option<ClearColor>,
    pub caps_lock_enabled: bool,
    pub keyboard_enabled: bool,
    pub keyboard_position: Option<WidgetPosition>,
    pub keyboard_background_color: ClearColor,
    pub keyboard_background_size: Option<i32>,
    pub keyboard_color: Option<ClearColor>,
    pub keyboard_size: Option<u32>,
    pub power_status_enabled: bool,
    pub power_status_position: Option<WidgetPosition>,
    pub battery_enabled: bool,
    pub battery_position: Option<WidgetPosition>,
    pub battery_color: Option<ClearColor>,
    pub battery_background_color: ClearColor,
    pub battery_background_size: Option<i32>,
    pub battery_size: Option<i32>,
    pub backdrops: Vec<Backdrop>,
    pub grid: Option<PreviewGrid>,
    pub weather_enabled: bool,
    pub weather_icon_enabled: bool,
    pub weather_icon_position: Option<WidgetPosition>,
    pub weather_icon_size: Option<i32>,
    pub weather_icon_opacity: Option<u8>,
    pub weather_temperature_enabled: bool,
    pub weather_temperature_color: Option<ClearColor>,
    pub weather_temperature_font_family: Option<String>,
    pub weather_temperature_font_weight: Option<u16>,
    pub weather_temperature_font_style: Option<FontStyle>,
    pub weather_temperature_letter_spacing: Option<u32>,
    pub weather_temperature_font_size: Option<u32>,
    pub weather_temperature_position: Option<WidgetPosition>,
    pub weather_location_enabled: bool,
    pub weather_location_color: Option<ClearColor>,
    pub weather_location_font_family: Option<String>,
    pub weather_location_font_weight: Option<u16>,
    pub weather_location_font_style: Option<FontStyle>,
    pub weather_location_font_size: Option<u32>,
    pub weather_location_position: Option<WidgetPosition>,
    pub now_playing_enabled: bool,
    pub now_playing_fade_duration_ms: Option<u64>,
    pub now_playing_artwork_enabled: bool,
    pub now_playing_artist_enabled: bool,
    pub now_playing_title_enabled: bool,
    pub now_playing_artwork_position: Option<WidgetPosition>,
    pub now_playing_artwork_size: Option<i32>,
    pub now_playing_artwork_radius: Option<i32>,
    pub now_playing_artwork_opacity: Option<u8>,
    pub now_playing_artist_position: Option<WidgetPosition>,
    pub now_playing_artist_width: Option<i32>,
    pub now_playing_artist_color: Option<ClearColor>,
    pub now_playing_artist_font_family: Option<String>,
    pub now_playing_artist_font_size: Option<u32>,
    pub now_playing_artist_font_weight: Option<u16>,
    pub now_playing_artist_font_style: Option<FontStyle>,
    pub now_playing_title_position: Option<WidgetPosition>,
    pub now_playing_title_width: Option<i32>,
    pub now_playing_title_color: Option<ClearColor>,
    pub now_playing_title_font_family: Option<String>,
    pub now_playing_title_font_size: Option<u32>,
    pub now_playing_title_font_weight: Option<u16>,
    pub now_playing_title_font_style: Option<FontStyle>,
    pub status_enabled: bool,
    pub status_color: Option<ClearColor>,
    pub status_pending_color: Option<ClearColor>,
    pub status_rejected_color: Option<ClearColor>,
    pub caps_lock_color: Option<ClearColor>,
    pub input_mask_color: Option<ClearColor>,
    pub foreground: ClearColor,
    pub muted: ClearColor,
    pub pending: ClearColor,
    pub rejected: ClearColor,
}

impl Default for ShellTheme {
    fn default() -> Self {
        Self::from_config(&AppConfig::default())
    }
}

fn resolve_position(
    position: WidgetPositionConfig,
    default_halign: HorizontalAlign,
    default_valign: VerticalAlign,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    if !position.is_specified() {
        return None;
    }

    Some(WidgetPosition {
        halign: position.halign.unwrap_or(default_halign),
        valign: position.valign.unwrap_or(default_valign),
        x: i32::from(position.x.unwrap_or(0)),
        y: i32::from(position.y.unwrap_or(0)),
        target: position
            .relative_to
            .as_deref()
            .and_then(|name| named_backdrops.get(name).copied())
            .map_or(WidgetPositionTarget::Screen, WidgetPositionTarget::Backdrop),
    })
}

fn resolve_clock_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.clock_position(),
        HorizontalAlign::Center,
        VerticalAlign::Top,
        named_backdrops,
    )
}

fn resolve_input_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.input_position(),
        HorizontalAlign::Center,
        VerticalAlign::Center,
        named_backdrops,
    )
}

fn resolve_status_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.status_position(),
        HorizontalAlign::Center,
        VerticalAlign::Center,
        named_backdrops,
    )
}

fn resolve_keyboard_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.keyboard_position(),
        HorizontalAlign::Right,
        VerticalAlign::Top,
        named_backdrops,
    )
}

fn resolve_battery_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.battery_position(),
        HorizontalAlign::Right,
        VerticalAlign::Top,
        named_backdrops,
    )
}

fn resolve_weather_icon_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.weather_icon_position(),
        HorizontalAlign::Left,
        VerticalAlign::Bottom,
        named_backdrops,
    )
}

fn resolve_weather_temperature_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.weather_temperature_position(),
        HorizontalAlign::Left,
        VerticalAlign::Bottom,
        named_backdrops,
    )
}

fn resolve_weather_location_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.weather_location_position(),
        HorizontalAlign::Left,
        VerticalAlign::Bottom,
        named_backdrops,
    )
}

fn resolve_power_status_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.power_status_position(),
        HorizontalAlign::Right,
        VerticalAlign::Top,
        named_backdrops,
    )
}

fn resolve_backdrops(config: &AppConfig) -> (Vec<Backdrop>, HashMap<String, usize>) {
    let mut backdrops = config
        .visuals
        .backdrop
        .iter()
        .filter(|backdrop| backdrop.enabled.unwrap_or(true))
        .map(|backdrop| {
            (
                backdrop.name.clone(),
                Backdrop {
                    mode: backdrop.mode.unwrap_or_default(),
                    color: to_color(backdrop.color.unwrap_or(config.visuals.panel)),
                    blur_strength: backdrop.blur_strength.unwrap_or(12).min(24),
                    radius: i32::from(backdrop.radius.unwrap_or(0)).clamp(0, 160),
                    border_color: backdrop.border_color.map(to_color),
                    border_width: i32::from(backdrop.border_width.unwrap_or(0)).clamp(0, 16),
                    full_width: backdrop.full_width.unwrap_or(false),
                    full_height: backdrop.full_height.unwrap_or(false),
                    width: i32::from(backdrop.width.unwrap_or(560)).max(1),
                    height: i32::from(backdrop.height.unwrap_or(600)).max(1),
                    position: WidgetPosition {
                        halign: backdrop.position.halign.unwrap_or(HorizontalAlign::Center),
                        valign: backdrop.position.valign.unwrap_or(VerticalAlign::Top),
                        x: i32::from(backdrop.position.x.unwrap_or(0)),
                        y: i32::from(backdrop.position.y.unwrap_or(0)),
                        target: WidgetPositionTarget::Screen,
                    },
                    z: i32::from(backdrop.z.unwrap_or(0)),
                },
            )
        })
        .collect::<Vec<_>>();
    backdrops.sort_by_key(|(_, backdrop)| backdrop.z);

    let mut named_backdrops = HashMap::new();
    for (index, (name, _)) in backdrops.iter().enumerate() {
        if let Some(name) = name.as_ref() {
            named_backdrops.entry(name.clone()).or_insert(index);
        }
    }

    (
        backdrops
            .into_iter()
            .map(|(_, backdrop)| backdrop)
            .collect(),
        named_backdrops,
    )
}

fn resolve_grid(config: &AppConfig) -> Option<PreviewGrid> {
    if !config.visuals.grid_enabled() {
        return None;
    }

    let GridVisualConfig {
        cell_size,
        color,
        major_every,
        major_color,
        ..
    } = config.visuals.grid.clone().unwrap_or_default();

    Some(PreviewGrid {
        cell_size: i32::from(cell_size.unwrap_or(40)).clamp(8, 240),
        color: to_color(color.unwrap_or(veila_common::RgbColor::rgba(255, 255, 255, 20))),
        major_every: i32::from(major_every.unwrap_or(4)).clamp(2, 12),
        major_color: to_color(
            major_color.unwrap_or(veila_common::RgbColor::rgba(255, 255, 255, 38)),
        ),
    })
}

fn resolve_now_playing_artwork_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.now_playing_artwork_position(),
        HorizontalAlign::Right,
        VerticalAlign::Bottom,
        named_backdrops,
    )
}

fn resolve_now_playing_artist_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.now_playing_artist_position(),
        HorizontalAlign::Right,
        VerticalAlign::Bottom,
        named_backdrops,
    )
}

fn resolve_now_playing_title_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.now_playing_title_position(),
        HorizontalAlign::Right,
        VerticalAlign::Bottom,
        named_backdrops,
    )
}

fn resolve_date_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.date_position(),
        HorizontalAlign::Center,
        VerticalAlign::Top,
        named_backdrops,
    )
}

fn resolve_avatar_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.avatar_position(),
        HorizontalAlign::Center,
        VerticalAlign::Center,
        named_backdrops,
    )
}

fn resolve_username_position(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.username_position(),
        HorizontalAlign::Center,
        VerticalAlign::Center,
        named_backdrops,
    )
}

impl ShellTheme {
    pub fn from_config(config: &AppConfig) -> Self {
        let (backdrops, named_backdrops) = resolve_backdrops(config);
        let clock_position = resolve_clock_position(config, &named_backdrops);
        let date_position = resolve_date_position(config, &named_backdrops);
        let avatar_position = resolve_avatar_position(config, &named_backdrops);
        let username_position = resolve_username_position(config, &named_backdrops);
        let keyboard_position = resolve_keyboard_position(config, &named_backdrops);
        let weather_icon_position = resolve_weather_icon_position(config, &named_backdrops);
        let weather_temperature_position =
            resolve_weather_temperature_position(config, &named_backdrops);
        let weather_location_position = resolve_weather_location_position(config, &named_backdrops);
        let power_status_position = resolve_power_status_position(config, &named_backdrops);
        let battery_position = resolve_battery_position(config, &named_backdrops);
        let grid = resolve_grid(config);
        let now_playing_artwork_position =
            resolve_now_playing_artwork_position(config, &named_backdrops);
        let now_playing_artist_position =
            resolve_now_playing_artist_position(config, &named_backdrops);
        let now_playing_title_position =
            resolve_now_playing_title_position(config, &named_backdrops);
        Self {
            background: to_color(config.background.color),
            avatar_enabled: config.visuals.avatar_enabled(),
            avatar_background: config
                .visuals
                .avatar_background_color()
                .map(to_color)
                .unwrap_or_else(|| to_color(config.visuals.panel)),
            input: to_color(config.visuals.input_background_color()),
            input_border: to_color(config.visuals.input_border_color()),
            input_font_family: config.visuals.input_font_family().map(str::to_owned),
            input_font_weight: config.visuals.input_font_weight(),
            input_font_style: config.visuals.input_font_style(),
            input_font_size: config.visuals.input_font_size().map(u32::from),
            input_reveal_on_interaction: config.visuals.input_reveal_on_interaction(),
            input_reveal_mode: config.visuals.input_reveal_mode(),
            input_reveal_hint: config.visuals.reveal_text(),
            reveal_enabled: config.visuals.reveal_enabled(),
            reveal_color: config.visuals.reveal_color().map(to_color),
            reveal_font_family: config.visuals.reveal_font_family().map(str::to_owned),
            reveal_font_weight: config.visuals.reveal_font_weight(),
            reveal_font_style: config.visuals.reveal_font_style(),
            reveal_font_size: config.visuals.reveal_font_size().map(u32::from),
            input_position: resolve_input_position(config, &named_backdrops),
            input_width: config.visuals.input_width().map(i32::from),
            input_height: config.visuals.input_height().map(i32::from),
            input_radius: i32::from(config.visuals.input_radius()),
            input_border_width: config.visuals.input_border_width().map(i32::from),
            avatar_size: config.visuals.avatar_size().map(i32::from),
            avatar_offset_y: Some(0),
            avatar_position,
            avatar_placeholder_padding: config.visuals.avatar_placeholder_padding().map(i32::from),
            avatar_icon_color: config.visuals.avatar_icon_color().map(to_color),
            avatar_ring_color: config.visuals.avatar_ring_color().map(to_color),
            avatar_ring_width: config.visuals.avatar_ring_width().map(i32::from),
            username_enabled: config.visuals.username_enabled(),
            username_font_family: config.visuals.username_font_family().map(str::to_owned),
            username_font_weight: config.visuals.username_font_weight(),
            username_font_style: config.visuals.username_font_style(),
            username_color: config.visuals.username_color().map(to_color),
            username_font_size: config.visuals.username_font_size().map(u32::from),
            username_offset_y: Some(0),
            username_position,
            avatar_gap: Some(24),
            username_gap: Some(28),
            status_position: resolve_status_position(config, &named_backdrops),
            status_mode: config.visuals.status_mode(),
            clock_gap: Some(20),
            clock_enabled: config.visuals.clock_enabled(),
            clock_alignment: ClockAlignment::TopCenter,
            clock_center_in_layer: false,
            clock_offset_x: Some(0),
            clock_offset_y: Some(0),
            clock_position,
            clock_font_family: config.visuals.clock_font_family().map(str::to_owned),
            clock_font_weight: config.visuals.clock_font_weight(),
            clock_font_style: config.visuals.clock_font_style(),
            clock_style: config.visuals.clock_style(),
            clock_format: config.visuals.clock_format(),
            clock_meridiem_font_size: config.visuals.clock_meridiem_font_size().map(u32::from),
            clock_meridiem_x: config.visuals.clock_meridiem_x().map(i32::from),
            clock_meridiem_y: config.visuals.clock_meridiem_y().map(i32::from),
            clock_color: config.visuals.clock_color().map(to_color),
            date_enabled: config.visuals.date_enabled(),
            date_font_family: config.visuals.date_font_family().map(str::to_owned),
            date_font_weight: config.visuals.date_font_weight(),
            date_font_style: config.visuals.date_font_style(),
            date_color: config.visuals.date_color().map(to_color),
            date_position,
            clock_font_size: config.visuals.clock_font_size().map(u32::from),
            date_font_size: config.visuals.date_font_size().map(u32::from),
            placeholder_enabled: config.visuals.placeholder_enabled(),
            placeholder_color: config.visuals.placeholder_color().map(to_color),
            eye_enabled: config.visuals.eye_enabled(),
            eye_icon_color: config.visuals.eye_icon_color().map(to_color),
            caps_lock_enabled: config.visuals.caps_lock_enabled(),
            keyboard_enabled: config.visuals.keyboard_enabled(),
            keyboard_position,
            keyboard_background_color: config
                .visuals
                .keyboard_background_color()
                .map(to_color)
                .unwrap_or_else(|| ClearColor::rgba(18, 22, 30, 82)),
            keyboard_background_size: config.visuals.keyboard_background_size().map(i32::from),
            keyboard_color: config.visuals.keyboard_color().map(to_color),
            keyboard_size: config.visuals.keyboard_size().map(u32::from),
            power_status_enabled: config.visuals.power_status_enabled(),
            power_status_position,
            battery_enabled: config.visuals.battery_enabled(),
            battery_position,
            battery_color: config.visuals.battery_color().map(to_color),
            battery_background_color: config
                .visuals
                .battery_background_color()
                .map(to_color)
                .unwrap_or_else(|| ClearColor::rgba(18, 22, 30, 82)),
            battery_background_size: config.visuals.battery_background_size().map(i32::from),
            battery_size: config.visuals.battery_size().map(i32::from),
            backdrops,
            grid,
            weather_enabled: config.visuals.weather_enabled(),
            weather_icon_enabled: config.visuals.weather_icon_enabled(),
            weather_icon_position,
            weather_icon_size: config.visuals.weather_icon_size().map(i32::from),
            weather_icon_opacity: config.visuals.weather_icon_opacity(),
            weather_temperature_enabled: config.visuals.weather_temperature_enabled(),
            weather_temperature_color: config.visuals.weather_temperature_color().map(to_color),
            weather_temperature_font_family: config
                .visuals
                .weather_temperature_font_family()
                .map(str::to_owned),
            weather_temperature_font_weight: config.visuals.weather_temperature_font_weight(),
            weather_temperature_font_style: config.visuals.weather_temperature_font_style(),
            weather_temperature_letter_spacing: config
                .visuals
                .weather_temperature_letter_spacing()
                .map(u32::from),
            weather_temperature_font_size: config
                .visuals
                .weather_temperature_font_size()
                .map(u32::from),
            weather_temperature_position,
            weather_location_enabled: config.visuals.weather_location_enabled(),
            weather_location_color: config.visuals.weather_location_color().map(to_color),
            weather_location_font_family: config
                .visuals
                .weather_location_font_family()
                .map(str::to_owned),
            weather_location_font_weight: config.visuals.weather_location_font_weight(),
            weather_location_font_style: config.visuals.weather_location_font_style(),
            weather_location_font_size: config.visuals.weather_location_font_size().map(u32::from),
            weather_location_position,
            now_playing_enabled: config.visuals.now_playing_enabled(),
            now_playing_fade_duration_ms: config
                .visuals
                .now_playing_fade_duration_ms()
                .map(u64::from),
            now_playing_artwork_enabled: config.visuals.now_playing_artwork_enabled(),
            now_playing_artist_enabled: config.visuals.now_playing_artist_enabled(),
            now_playing_title_enabled: config.visuals.now_playing_title_enabled(),
            now_playing_artwork_position,
            now_playing_artwork_size: config.visuals.now_playing_artwork_size().map(i32::from),
            now_playing_artwork_radius: config.visuals.now_playing_artwork_radius().map(i32::from),
            now_playing_artwork_opacity: config.visuals.now_playing_artwork_opacity(),
            now_playing_artist_position,
            now_playing_artist_width: config.visuals.now_playing_artist_width().map(i32::from),
            now_playing_artist_color: config.visuals.now_playing_artist_color().map(to_color),
            now_playing_artist_font_family: config
                .visuals
                .now_playing_artist_font_family()
                .map(str::to_owned),
            now_playing_artist_font_size: config
                .visuals
                .now_playing_artist_font_size()
                .map(u32::from),
            now_playing_artist_font_weight: config.visuals.now_playing_artist_font_weight(),
            now_playing_artist_font_style: config.visuals.now_playing_artist_font_style(),
            now_playing_title_position,
            now_playing_title_width: config.visuals.now_playing_title_width().map(i32::from),
            now_playing_title_color: config.visuals.now_playing_title_color().map(to_color),
            now_playing_title_font_family: config
                .visuals
                .now_playing_title_font_family()
                .map(str::to_owned),
            now_playing_title_font_size: config
                .visuals
                .now_playing_title_font_size()
                .map(u32::from),
            now_playing_title_font_weight: config.visuals.now_playing_title_font_weight(),
            now_playing_title_font_style: config.visuals.now_playing_title_font_style(),
            status_enabled: config.visuals.status_enabled(),
            status_color: config.visuals.status_color().map(to_color),
            status_pending_color: config.visuals.status_pending_color().map(to_color),
            status_rejected_color: config.visuals.status_rejected_color().map(to_color),
            caps_lock_color: config.visuals.caps_lock_color().map(to_color),
            input_mask_color: config.visuals.input_mask_color().map(to_color),
            foreground: to_color(config.visuals.foreground_color()),
            muted: to_color(config.visuals.muted_color()),
            pending: to_color(config.visuals.pending_color()),
            rejected: to_color(config.visuals.rejected_color()),
        }
    }
}
