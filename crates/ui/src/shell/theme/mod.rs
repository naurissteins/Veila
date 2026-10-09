mod auth;
#[cfg(test)]
mod auth_tests;
mod color;
mod font_warmup;
mod header;
#[cfg(test)]
mod header_tests;
mod identity;
#[cfg(test)]
mod identity_tests;
mod media;
#[cfg(test)]
mod media_tests;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use veila_common::{
    AppConfig, BackdropMode, BackdropShowWhen, FontStyle, GridVisualConfig, HorizontalAlign,
    LayerKind, PowerAction, VerticalAlign, WidgetPositionConfig,
};
use veila_renderer::{ClearColor, RenderScale};

use self::color::to_color;
use super::PreviewGrid;

pub use auth::{CapsLockTheme, EyeTheme, InputTheme, PlaceholderTheme, RevealTheme, StatusTheme};
pub use header::{ClockTheme, DateTheme};
pub use identity::{AvatarTheme, UsernameTheme};
pub use media::NowPlayingTheme;

// Missing surface colors retain the pre-theme fallback independently of config keys.
const DEFAULT_SURFACE_COLOR: veila_common::RgbColor = veila_common::RgbColor::rgb(22, 28, 38);

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
    pub show_when: BackdropShowWhen,
    pub color: ClearColor,
    pub blur_strength: u8,
    pub radius: i32,
    pub border_color: Option<ClearColor>,
    pub border_width: i32,
    pub full_width: bool,
    pub full_height: bool,
    pub inset_top: i32,
    pub inset_bottom: i32,
    pub inset_left: i32,
    pub inset_right: i32,
    pub width: i32,
    pub height: i32,
    pub rotate: i16,
    pub position: WidgetPosition,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualLayer {
    pub kind: LayerKind,
    pub text: String,
    pub color: ClearColor,
    pub background_color: Option<ClearColor>,
    pub font_family: Option<String>,
    pub font_weight: Option<u16>,
    pub font_style: Option<FontStyle>,
    pub font_size: u32,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub padding: i32,
    pub radius: i32,
    pub position: WidgetPosition,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerButton {
    pub action: PowerAction,
    pub enabled: bool,
    pub position: Option<WidgetPosition>,
    pub background_color: ClearColor,
    pub background_size: Option<i32>,
    pub radius: Option<i32>,
    pub color: Option<ClearColor>,
    pub size: i32,
    pub confirm: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellTheme {
    pub clock: ClockTheme,
    pub date: DateTheme,
    pub avatar: AvatarTheme,
    pub username: UsernameTheme,
    pub input: InputTheme,
    pub reveal: RevealTheme,
    pub placeholder: PlaceholderTheme,
    pub eye: EyeTheme,
    pub caps_lock: CapsLockTheme,
    pub status: StatusTheme,
    pub background: ClearColor,
    pub keyboard_enabled: bool,
    pub keyboard_position: Option<WidgetPosition>,
    pub keyboard_background_color: ClearColor,
    pub keyboard_background_size: Option<i32>,
    pub keyboard_radius: Option<i32>,
    pub keyboard_color: Option<ClearColor>,
    pub keyboard_size: Option<u32>,
    pub power_status_enabled: bool,
    pub power_status_position: Option<WidgetPosition>,
    pub power_buttons: [PowerButton; 3],
    pub battery_enabled: bool,
    pub battery_position: Option<WidgetPosition>,
    pub battery_color: Option<ClearColor>,
    pub battery_background_color: ClearColor,
    pub battery_background_size: Option<i32>,
    pub battery_radius: Option<i32>,
    pub battery_size: Option<i32>,
    pub backdrops: Vec<Backdrop>,
    pub layers: Vec<VisualLayer>,
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
    pub now_playing: NowPlayingTheme,
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

impl ShellTheme {
    #[cfg(test)]
    pub(crate) fn scaled_for_render(&self, scale: u32) -> Self {
        self.scaled_for_render_at(RenderScale::from_integer(scale))
    }

    pub(crate) fn scaled_for_render_at(&self, scale: RenderScale) -> Self {
        if scale == RenderScale::ONE {
            return self.clone();
        }

        let mut theme = self.clone();
        theme.input.scale_for_render(scale);
        theme.reveal.scale_for_render(scale);
        theme.status.scale_for_render(scale);
        theme.avatar.scale_for_render(scale);
        theme.username.scale_for_render(scale);
        theme.clock.scale_for_render(scale);
        theme.date.scale_for_render(scale);
        theme.keyboard_position = theme
            .keyboard_position
            .map(|position| scale_position(position, scale));
        theme.keyboard_background_size = scale_i32_opt(theme.keyboard_background_size, scale);
        theme.keyboard_radius = scale_i32_opt(theme.keyboard_radius, scale);
        theme.keyboard_size = scale_u32_opt(theme.keyboard_size, scale);
        theme.power_status_position = theme
            .power_status_position
            .map(|position| scale_position(position, scale));
        theme.power_buttons = theme
            .power_buttons
            .map(|button| scale_power_button(button, scale));
        theme.battery_position = theme
            .battery_position
            .map(|position| scale_position(position, scale));
        theme.battery_background_size = scale_i32_opt(theme.battery_background_size, scale);
        theme.battery_radius = scale_i32_opt(theme.battery_radius, scale);
        theme.battery_size = scale_i32_opt(theme.battery_size, scale);
        theme.backdrops = theme
            .backdrops
            .into_iter()
            .map(|backdrop| scale_backdrop(backdrop, scale))
            .collect();
        theme.layers = theme
            .layers
            .into_iter()
            .map(|layer| scale_visual_layer(layer, scale))
            .collect();
        theme.grid = theme.grid.map(|grid| scale_grid(grid, scale));
        theme.weather_icon_position = theme
            .weather_icon_position
            .map(|position| scale_position(position, scale));
        theme.weather_icon_size = scale_i32_opt(theme.weather_icon_size, scale);
        theme.weather_temperature_font_size =
            scale_u32_opt(theme.weather_temperature_font_size, scale);
        theme.weather_temperature_letter_spacing =
            scale_u32_opt(theme.weather_temperature_letter_spacing, scale);
        theme.weather_temperature_position = theme
            .weather_temperature_position
            .map(|position| scale_position(position, scale));
        theme.weather_location_font_size = scale_u32_opt(theme.weather_location_font_size, scale);
        theme.weather_location_position = theme
            .weather_location_position
            .map(|position| scale_position(position, scale));
        theme.now_playing.scale_for_render(scale);
        theme
    }
}

fn scale_u32_opt(value: Option<u32>, scale: RenderScale) -> Option<u32> {
    value.map(|value| scale.apply_u32(value))
}

fn scale_i32_opt(value: Option<i32>, scale: RenderScale) -> Option<i32> {
    value.map(|value| scale_i32(value, scale))
}

fn scale_i32(value: i32, scale: RenderScale) -> i32 {
    scale.apply_i32(value)
}

fn scale_position(position: WidgetPosition, scale: RenderScale) -> WidgetPosition {
    WidgetPosition {
        x: scale_i32(position.x, scale),
        y: scale_i32(position.y, scale),
        ..position
    }
}

fn scale_backdrop(mut backdrop: Backdrop, scale: RenderScale) -> Backdrop {
    backdrop.radius = scale_i32(backdrop.radius, scale);
    backdrop.border_width = scale_i32(backdrop.border_width, scale);
    backdrop.inset_top = scale_i32(backdrop.inset_top, scale);
    backdrop.inset_bottom = scale_i32(backdrop.inset_bottom, scale);
    backdrop.inset_left = scale_i32(backdrop.inset_left, scale);
    backdrop.inset_right = scale_i32(backdrop.inset_right, scale);
    backdrop.width = scale_i32(backdrop.width, scale);
    backdrop.height = scale_i32(backdrop.height, scale);
    backdrop.position = scale_position(backdrop.position, scale);
    backdrop
}

fn scale_visual_layer(mut layer: VisualLayer, scale: RenderScale) -> VisualLayer {
    layer.font_size = scale.apply_u32(layer.font_size);
    layer.width = scale_i32_opt(layer.width, scale);
    layer.height = scale_i32_opt(layer.height, scale);
    layer.padding = scale_i32(layer.padding, scale);
    layer.radius = scale_i32(layer.radius, scale);
    layer.position = scale_position(layer.position, scale);
    layer
}

fn scale_power_button(mut button: PowerButton, scale: RenderScale) -> PowerButton {
    button.position = button
        .position
        .map(|position| scale_position(position, scale));
    button.background_size = scale_i32_opt(button.background_size, scale);
    button.radius = scale_i32_opt(button.radius, scale);
    button.size = scale_i32(button.size, scale);
    button
}

fn scale_grid(mut grid: PreviewGrid, scale: RenderScale) -> PreviewGrid {
    grid.cell_size = scale_i32(grid.cell_size, scale);
    grid.major_every = grid.major_every.max(1);
    grid
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

fn resolve_power_button_position(
    config: &AppConfig,
    action: PowerAction,
    named_backdrops: &HashMap<String, usize>,
) -> Option<WidgetPosition> {
    resolve_position(
        config.visuals.power_button_position(action),
        HorizontalAlign::Right,
        VerticalAlign::Top,
        named_backdrops,
    )
}

fn resolve_power_button(
    config: &AppConfig,
    action: PowerAction,
    named_backdrops: &HashMap<String, usize>,
) -> PowerButton {
    PowerButton {
        action,
        enabled: config.visuals.power_button_enabled(action),
        position: resolve_power_button_position(config, action, named_backdrops),
        background_color: config
            .visuals
            .power_button_background_color(action)
            .map(to_color)
            .unwrap_or_else(|| ClearColor::rgba(18, 22, 30, 82)),
        background_size: config
            .visuals
            .power_button_background_size(action)
            .map(i32::from),
        radius: config
            .visuals
            .power_button_radius(action)
            .map(|radius| i32::from(radius).clamp(0, 160)),
        color: config.visuals.power_button_color(action).map(to_color),
        size: i32::from(config.visuals.power_button_size(action).unwrap_or(20)).clamp(12, 96),
        confirm: config.visuals.power_button_confirm(action),
    }
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
                    show_when: backdrop.show_when.unwrap_or_default(),
                    color: to_color(backdrop.color.unwrap_or(DEFAULT_SURFACE_COLOR)),
                    blur_strength: backdrop.blur_strength.unwrap_or(12).min(24),
                    radius: i32::from(backdrop.radius.unwrap_or(0)).clamp(0, 160),
                    border_color: backdrop.border_color.map(to_color),
                    border_width: i32::from(backdrop.border_width.unwrap_or(0)).clamp(0, 16),
                    full_width: backdrop.full_width.unwrap_or(false),
                    full_height: backdrop.full_height.unwrap_or(false),
                    inset_top: i32::from(backdrop.inset_top.unwrap_or(0)).clamp(0, 4_096),
                    inset_bottom: i32::from(backdrop.inset_bottom.unwrap_or(0)).clamp(0, 4_096),
                    inset_left: i32::from(backdrop.inset_left.unwrap_or(0)).clamp(0, 4_096),
                    inset_right: i32::from(backdrop.inset_right.unwrap_or(0)).clamp(0, 4_096),
                    width: i32::from(backdrop.width.unwrap_or(560)).max(1),
                    height: i32::from(backdrop.height.unwrap_or(600)).max(1),
                    rotate: normalize_rotation(backdrop.rotate.unwrap_or(0)),
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

fn normalize_rotation(degrees: i16) -> i16 {
    degrees.rem_euclid(360)
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

fn resolve_layers(
    config: &AppConfig,
    named_backdrops: &HashMap<String, usize>,
) -> Vec<VisualLayer> {
    let mut layers = config
        .visuals
        .layer
        .iter()
        .filter(|layer| layer.enabled.unwrap_or(true))
        .filter_map(|layer| {
            let text = layer.text.as_deref().unwrap_or_default().trim();
            if text.is_empty() {
                return None;
            }

            Some(VisualLayer {
                kind: layer.kind.unwrap_or_default(),
                text: text.to_owned(),
                color: to_color(layer.color.unwrap_or(config.visuals.foreground_color())),
                background_color: layer.background_color.map(to_color),
                font_family: layer.font_family.clone(),
                font_weight: layer.font_weight,
                font_style: layer.font_style,
                font_size: u32::from(layer.font_size.unwrap_or(24)).clamp(1, 512),
                width: layer.width.map(|width| i32::from(width).max(1)),
                height: layer.height.map(|height| i32::from(height).max(1)),
                padding: i32::from(layer.padding.unwrap_or(0)).clamp(0, 512),
                radius: i32::from(layer.radius.unwrap_or(0)).clamp(0, 512),
                position: resolve_position(
                    layer.position.clone(),
                    HorizontalAlign::Center,
                    VerticalAlign::Center,
                    named_backdrops,
                )
                .unwrap_or(WidgetPosition {
                    halign: HorizontalAlign::Center,
                    valign: VerticalAlign::Center,
                    x: 0,
                    y: 0,
                    target: WidgetPositionTarget::Screen,
                }),
                z: i32::from(layer.z.unwrap_or(0)),
            })
        })
        .collect::<Vec<_>>();
    layers.sort_by_key(|layer| layer.z);
    layers
}

impl ShellTheme {
    pub fn from_config(config: &AppConfig) -> Self {
        let (backdrops, named_backdrops) = resolve_backdrops(config);
        let layers = resolve_layers(config, &named_backdrops);
        let keyboard_position = resolve_keyboard_position(config, &named_backdrops);
        let weather_icon_position = resolve_weather_icon_position(config, &named_backdrops);
        let weather_temperature_position =
            resolve_weather_temperature_position(config, &named_backdrops);
        let weather_location_position = resolve_weather_location_position(config, &named_backdrops);
        let power_status_position = resolve_power_status_position(config, &named_backdrops);
        let power_buttons = [
            resolve_power_button(config, PowerAction::Suspend, &named_backdrops),
            resolve_power_button(config, PowerAction::Reboot, &named_backdrops),
            resolve_power_button(config, PowerAction::Poweroff, &named_backdrops),
        ];
        let battery_position = resolve_battery_position(config, &named_backdrops);
        let grid = resolve_grid(config);
        Self {
            clock: ClockTheme::from_config(config, &named_backdrops),
            date: DateTheme::from_config(config, &named_backdrops),
            avatar: AvatarTheme::from_config(config, &named_backdrops),
            username: UsernameTheme::from_config(config, &named_backdrops),
            input: InputTheme::from_config(config, &named_backdrops),
            reveal: RevealTheme::from_config(config),
            placeholder: PlaceholderTheme::from_config(config),
            eye: EyeTheme::from_config(config),
            caps_lock: CapsLockTheme::from_config(config),
            status: StatusTheme::from_config(config, &named_backdrops),
            background: to_color(config.background.color),
            keyboard_enabled: config.visuals.keyboard_enabled(),
            keyboard_position,
            keyboard_background_color: config
                .visuals
                .keyboard_background_color()
                .map(to_color)
                .unwrap_or_else(|| ClearColor::rgba(18, 22, 30, 82)),
            keyboard_background_size: config.visuals.keyboard_background_size().map(i32::from),
            keyboard_radius: config
                .visuals
                .keyboard_radius()
                .map(|radius| i32::from(radius).clamp(0, 160)),
            keyboard_color: config.visuals.keyboard_color().map(to_color),
            keyboard_size: config.visuals.keyboard_size().map(u32::from),
            power_status_enabled: config.visuals.power_status_enabled(),
            power_status_position,
            power_buttons,
            battery_enabled: config.visuals.battery_enabled(),
            battery_position,
            battery_color: config.visuals.battery_color().map(to_color),
            battery_background_color: config
                .visuals
                .battery_background_color()
                .map(to_color)
                .unwrap_or_else(|| ClearColor::rgba(18, 22, 30, 82)),
            battery_background_size: config.visuals.battery_background_size().map(i32::from),
            battery_radius: config
                .visuals
                .battery_radius()
                .map(|radius| i32::from(radius).clamp(0, 160)),
            battery_size: config.visuals.battery_size().map(i32::from),
            backdrops,
            layers,
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
            now_playing: NowPlayingTheme::from_config(config, &named_backdrops),
            foreground: to_color(config.visuals.foreground_color()),
            muted: to_color(config.visuals.muted_color()),
            pending: to_color(config.visuals.pending_color()),
            rejected: to_color(config.visuals.rejected_color()),
        }
    }
}
