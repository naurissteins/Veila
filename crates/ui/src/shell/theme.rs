use veila_common::AppConfig;
use veila_renderer::ClearColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellTheme {
    pub background: ClearColor,
    pub panel: ClearColor,
    pub panel_border: ClearColor,
    pub input: ClearColor,
    pub input_border: ClearColor,
    pub input_radius: i32,
    pub avatar_size: Option<i32>,
    pub avatar_placeholder_padding: Option<i32>,
    pub avatar_ring_width: Option<i32>,
    pub avatar_background_opacity: Option<u8>,
    pub username_opacity: Option<u8>,
    pub username_size: Option<u32>,
    pub clock_opacity: Option<u8>,
    pub date_opacity: Option<u8>,
    pub clock_size: Option<u32>,
    pub date_size: Option<u32>,
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
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            background: to_color(config.background.color),
            panel: to_color(config.visuals.panel),
            panel_border: to_color(config.visuals.panel_border),
            input: to_color_with_opacity(config.visuals.input, config.visuals.input_opacity),
            input_border: to_color_with_opacity(
                config.visuals.input_border,
                config.visuals.input_border_opacity,
            ),
            input_radius: i32::from(config.visuals.input_radius),
            avatar_size: config.visuals.avatar_size.map(i32::from),
            avatar_placeholder_padding: config.visuals.avatar_placeholder_padding.map(i32::from),
            avatar_ring_width: config.visuals.avatar_ring_width.map(i32::from),
            avatar_background_opacity: config.visuals.avatar_background_opacity,
            username_opacity: config.visuals.username_opacity,
            username_size: config.visuals.username_size.map(u32::from),
            clock_opacity: config.visuals.clock_opacity,
            date_opacity: config.visuals.date_opacity,
            clock_size: config.visuals.clock_size.map(u32::from),
            date_size: config.visuals.date_size.map(u32::from),
            foreground: to_color(config.visuals.foreground),
            muted: to_color(config.visuals.muted),
            pending: to_color(config.visuals.pending),
            rejected: to_color(config.visuals.rejected),
        }
    }
}

fn to_color(color: veila_common::RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}

fn to_color_with_opacity(color: veila_common::RgbColor, opacity_percent: Option<u8>) -> ClearColor {
    let color = to_color(color);
    let Some(opacity_percent) = opacity_percent else {
        return color;
    };

    color.with_alpha(alpha_from_percent(opacity_percent))
}

fn alpha_from_percent(percent: u8) -> u8 {
    ((u16::from(percent.min(100)) * 255 + 50) / 100) as u8
}

#[cfg(test)]
mod tests {
    use veila_common::{AppConfig, ConfigColor};

    use super::ShellTheme;

    #[test]
    fn input_opacity_overrides_embedded_alpha() {
        let mut config = AppConfig::default();
        config.visuals.input = ConfigColor::rgba(255, 255, 255, 200);
        config.visuals.input_opacity = Some(10);
        config.visuals.input_border = ConfigColor::rgba(255, 255, 255, 180);
        config.visuals.input_border_opacity = Some(12);
        config.visuals.avatar_size = Some(92);
        config.visuals.avatar_placeholder_padding = Some(14);
        config.visuals.avatar_ring_width = Some(3);
        config.visuals.avatar_background_opacity = Some(36);
        config.visuals.username_opacity = Some(72);
        config.visuals.username_size = Some(3);
        config.visuals.clock_opacity = Some(96);
        config.visuals.date_opacity = Some(74);
        config.visuals.clock_size = Some(4);
        config.visuals.date_size = Some(3);

        let theme = ShellTheme::from_config(&config);

        assert_eq!(theme.input.alpha, 26);
        assert_eq!(theme.input_border.alpha, 31);
        assert_eq!(theme.avatar_size, Some(92));
        assert_eq!(theme.avatar_placeholder_padding, Some(14));
        assert_eq!(theme.avatar_ring_width, Some(3));
        assert_eq!(theme.avatar_background_opacity, Some(36));
        assert_eq!(theme.username_opacity, Some(72));
        assert_eq!(theme.username_size, Some(3));
        assert_eq!(theme.clock_opacity, Some(96));
        assert_eq!(theme.date_opacity, Some(74));
        assert_eq!(theme.clock_size, Some(4));
        assert_eq!(theme.date_size, Some(3));
    }
}
