use super::{
    SceneTextInputs, ShellState, TextLayoutCache, layout::SceneMetrics, model::LayoutRole,
};
use crate::shell::{ShellStatus, ShellTheme};
use veila_common::{
    ClockStyle, InputAlignment, LayerAlignment, LayerMode, LayerVerticalAlignment,
    WeatherAlignment, WeatherCondition, WeatherSnapshot, WeatherUnit,
};
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    text::{TextStyle, bundled_clock_font_family},
};

mod auth_style_tests;
mod header_style_tests;
mod layout_tests;
mod text_cache_tests;
mod widget_style_tests;
