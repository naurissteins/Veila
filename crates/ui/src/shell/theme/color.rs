use veila_common::RgbColor;
use veila_renderer::ClearColor;

pub(super) fn to_color(color: RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}

pub(super) fn to_color_with_opacity(color: RgbColor, opacity_percent: Option<u8>) -> ClearColor {
    let color = to_color(color);
    let Some(opacity_percent) = opacity_percent else {
        return color;
    };

    color.with_alpha(alpha_from_percent(opacity_percent))
}

fn alpha_from_percent(percent: u8) -> u8 {
    ((u16::from(percent.min(100)) * 255 + 50) / 100) as u8
}
