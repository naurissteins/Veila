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
            input: to_color(config.visuals.input),
            input_border: to_color(config.visuals.input_border),
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
