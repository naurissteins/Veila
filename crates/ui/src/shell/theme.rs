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

        let theme = ShellTheme::from_config(&config);

        assert_eq!(theme.input.alpha, 26);
        assert_eq!(theme.input_border.alpha, 31);
    }
}
