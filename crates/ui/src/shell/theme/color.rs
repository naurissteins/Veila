use veila_common::RgbColor;
use veila_renderer::ClearColor;

pub(super) fn to_color(color: RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}
