use veila_common::AppConfig;
use veila_renderer::ClearColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellTheme {
    pub background: ClearColor,
    pub avatar_background: ClearColor,
    pub input: ClearColor,
    pub input_border: ClearColor,
    pub input_width: Option<i32>,
    pub input_height: Option<i32>,
    pub input_radius: i32,
    pub input_border_width: Option<i32>,
    pub avatar_size: Option<i32>,
    pub avatar_placeholder_padding: Option<i32>,
    pub avatar_icon_color: Option<ClearColor>,
    pub avatar_ring_color: Option<ClearColor>,
    pub avatar_ring_width: Option<i32>,
    pub avatar_background_opacity: Option<u8>,
    pub username_color: Option<ClearColor>,
    pub username_opacity: Option<u8>,
    pub username_size: Option<u32>,
    pub avatar_gap: Option<i32>,
    pub username_gap: Option<i32>,
    pub status_gap: Option<i32>,
    pub clock_color: Option<ClearColor>,
    pub clock_opacity: Option<u8>,
    pub date_color: Option<ClearColor>,
    pub date_opacity: Option<u8>,
    pub clock_size: Option<u32>,
    pub date_size: Option<u32>,
    pub placeholder_color: Option<ClearColor>,
    pub placeholder_opacity: Option<u8>,
    pub eye_icon_color: Option<ClearColor>,
    pub eye_icon_opacity: Option<u8>,
    pub status_color: Option<ClearColor>,
    pub status_opacity: Option<u8>,
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
            avatar_background: config
                .visuals
                .avatar_background_color
                .map(to_color)
                .unwrap_or_else(|| to_color(config.visuals.panel)),
            input: to_color_with_opacity(config.visuals.input, config.visuals.input_opacity),
            input_border: to_color_with_opacity(
                config.visuals.input_border,
                config.visuals.input_border_opacity,
            ),
            input_width: config.visuals.input_width.map(i32::from),
            input_height: config.visuals.input_height.map(i32::from),
            input_radius: i32::from(config.visuals.input_radius),
            input_border_width: config.visuals.input_border_width.map(i32::from),
            avatar_size: config.visuals.avatar_size.map(i32::from),
            avatar_placeholder_padding: config.visuals.avatar_placeholder_padding.map(i32::from),
            avatar_icon_color: config.visuals.avatar_icon_color.map(to_color),
            avatar_ring_color: config.visuals.avatar_ring_color.map(to_color),
            avatar_ring_width: config.visuals.avatar_ring_width.map(i32::from),
            avatar_background_opacity: config.visuals.avatar_background_opacity,
            username_color: config.visuals.username_color.map(to_color),
            username_opacity: config.visuals.username_opacity,
            username_size: config.visuals.username_size.map(u32::from),
            avatar_gap: config.visuals.avatar_gap.map(i32::from),
            username_gap: config.visuals.username_gap.map(i32::from),
            status_gap: config.visuals.status_gap.map(i32::from),
            clock_color: config.visuals.clock_color.map(to_color),
            clock_opacity: config.visuals.clock_opacity,
            date_color: config.visuals.date_color.map(to_color),
            date_opacity: config.visuals.date_opacity,
            clock_size: config.visuals.clock_size.map(u32::from),
            date_size: config.visuals.date_size.map(u32::from),
            placeholder_color: config.visuals.placeholder_color.map(to_color),
            placeholder_opacity: config.visuals.placeholder_opacity,
            eye_icon_color: config.visuals.eye_icon_color.map(to_color),
            eye_icon_opacity: config.visuals.eye_icon_opacity,
            status_color: config.visuals.status_color.map(to_color),
            status_opacity: config.visuals.status_opacity,
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
    use veila_renderer::ClearColor;

    use super::ShellTheme;

    #[test]
    fn input_opacity_overrides_embedded_alpha() {
        let mut config = AppConfig::default();
        config.visuals.input = ConfigColor::rgba(255, 255, 255, 200);
        config.visuals.input_opacity = Some(10);
        config.visuals.input_border = ConfigColor::rgba(255, 255, 255, 180);
        config.visuals.input_border_opacity = Some(12);
        config.visuals.avatar_background_color = Some(ConfigColor::rgb(24, 30, 42));
        config.visuals.input_width = Some(280);
        config.visuals.input_height = Some(54);
        config.visuals.avatar_size = Some(92);
        config.visuals.input_border_width = Some(3);
        config.visuals.avatar_placeholder_padding = Some(14);
        config.visuals.avatar_icon_color = Some(ConfigColor::rgb(232, 238, 249));
        config.visuals.avatar_ring_color = Some(ConfigColor::rgb(148, 178, 255));
        config.visuals.avatar_ring_width = Some(3);
        config.visuals.avatar_background_opacity = Some(36);
        config.visuals.username_color = Some(ConfigColor::rgb(215, 227, 255));
        config.visuals.username_opacity = Some(72);
        config.visuals.username_size = Some(3);
        config.visuals.avatar_gap = Some(14);
        config.visuals.username_gap = Some(28);
        config.visuals.status_gap = Some(18);
        config.visuals.clock_color = Some(ConfigColor::rgb(248, 251, 255));
        config.visuals.clock_opacity = Some(96);
        config.visuals.date_color = Some(ConfigColor::rgb(200, 212, 236));
        config.visuals.date_opacity = Some(74);
        config.visuals.clock_size = Some(4);
        config.visuals.date_size = Some(3);
        config.visuals.placeholder_color = Some(ConfigColor::rgb(134, 148, 180));
        config.visuals.placeholder_opacity = Some(60);
        config.visuals.eye_icon_color = Some(ConfigColor::rgb(244, 248, 255));
        config.visuals.eye_icon_opacity = Some(72);
        config.visuals.status_color = Some(ConfigColor::rgb(255, 224, 160));
        config.visuals.status_opacity = Some(88);

        let theme = ShellTheme::from_config(&config);

        assert_eq!(theme.input.alpha, 26);
        assert_eq!(theme.input_border.alpha, 31);
        assert_eq!(theme.avatar_background, ClearColor::opaque(24, 30, 42));
        assert_eq!(theme.input_width, Some(280));
        assert_eq!(theme.input_height, Some(54));
        assert_eq!(theme.input_border_width, Some(3));
        assert_eq!(theme.avatar_size, Some(92));
        assert_eq!(theme.avatar_placeholder_padding, Some(14));
        assert_eq!(
            theme.avatar_icon_color,
            Some(ClearColor::opaque(232, 238, 249))
        );
        assert_eq!(
            theme.avatar_ring_color,
            Some(ClearColor::opaque(148, 178, 255))
        );
        assert_eq!(theme.avatar_ring_width, Some(3));
        assert_eq!(theme.avatar_background_opacity, Some(36));
        assert_eq!(
            theme.username_color,
            Some(ClearColor::opaque(215, 227, 255))
        );
        assert_eq!(theme.username_opacity, Some(72));
        assert_eq!(theme.username_size, Some(3));
        assert_eq!(theme.avatar_gap, Some(14));
        assert_eq!(theme.username_gap, Some(28));
        assert_eq!(theme.status_gap, Some(18));
        assert_eq!(theme.clock_color, Some(ClearColor::opaque(248, 251, 255)));
        assert_eq!(theme.clock_opacity, Some(96));
        assert_eq!(theme.date_color, Some(ClearColor::opaque(200, 212, 236)));
        assert_eq!(theme.date_opacity, Some(74));
        assert_eq!(theme.clock_size, Some(4));
        assert_eq!(theme.date_size, Some(3));
        assert_eq!(
            theme.placeholder_color,
            Some(ClearColor::opaque(134, 148, 180))
        );
        assert_eq!(theme.placeholder_opacity, Some(60));
        assert_eq!(
            theme.eye_icon_color,
            Some(ClearColor::opaque(244, 248, 255))
        );
        assert_eq!(theme.eye_icon_opacity, Some(72));
        assert_eq!(theme.status_color, Some(ClearColor::opaque(255, 224, 160)));
        assert_eq!(theme.status_opacity, Some(88));
    }

    #[test]
    fn avatar_background_falls_back_to_legacy_panel_color() {
        let mut config = AppConfig::default();
        config.visuals.panel = ConfigColor::rgb(31, 39, 52);

        let theme = ShellTheme::from_config(&config);

        assert_eq!(theme.avatar_background, ClearColor::opaque(31, 39, 52));
    }
}
