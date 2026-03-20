use kwylock_renderer::{
    ClearColor,
    panel::{PanelBodyStyle, PanelHeaderStyle},
    shape::{BorderStyle, BoxShadow, BoxStyle},
};

use crate::ShellTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypographyScale {
    pub hint_scale: u32,
    pub hint_min_scale: u32,
    pub status_scale: u32,
    pub status_min_scale: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellVisualStyle {
    pub panel_width: i32,
    pub panel_style: BoxStyle,
    pub input_style: BoxStyle,
    pub header_style: PanelHeaderStyle,
    pub body_style: PanelBodyStyle,
    pub typography: TypographyScale,
}

pub fn visual_style_for_surface(
    theme: &ShellTheme,
    surface_width: i32,
    accent: ClearColor,
    focused: bool,
) -> ShellVisualStyle {
    let panel_width = ((surface_width * 11) / 28).clamp(360, 640);
    let typography = if panel_width >= 560 {
        TypographyScale {
            hint_scale: 3,
            hint_min_scale: 2,
            status_scale: 2,
            status_min_scale: 1,
        }
    } else {
        TypographyScale {
            hint_scale: 2,
            hint_min_scale: 1,
            status_scale: 1,
            status_min_scale: 1,
        }
    };

    let panel_style = BoxStyle::new(with_alpha(theme.panel, 232))
        .with_radius(24)
        .with_border(BorderStyle::new(with_alpha(theme.panel_border, 176), 1))
        .with_shadow(BoxShadow::new(ClearColor::rgba(2, 4, 8, 118), 26, 0, 0, 16));
    let input_style = BoxStyle::new(with_alpha(theme.input, 222))
        .with_radius(18)
        .with_border(BorderStyle::new(
            with_alpha(if focused { accent } else { theme.input_border }, 208),
            2,
        ))
        .with_shadow(BoxShadow::new(ClearColor::rgba(0, 0, 0, 64), 12, 0, 0, 6));

    ShellVisualStyle {
        panel_width,
        panel_style,
        input_style,
        header_style: PanelHeaderStyle::new(with_alpha(accent, 232)),
        body_style: PanelBodyStyle::new(),
        typography,
    }
}

fn with_alpha(color: ClearColor, alpha: u8) -> ClearColor {
    ClearColor::rgba(color.red, color.green, color.blue, alpha)
}
