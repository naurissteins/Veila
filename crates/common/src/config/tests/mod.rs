use std::fs;

use super::{
    AppConfig, BackgroundMode, CenterStackOrder, CenterStackStyle, ClockAlignment, ClockFormat,
    ClockStyle, FontStyle, InputAlignment, InputVisualEntry, LayerAlignment, LayerHeight,
    LayerHeightKeyword, LayerMode, LayerStyle, LayerVerticalAlignment, LayerWidth,
    LayerWidthKeyword, RgbColor, WeatherAlignment, WeatherUnit, active_theme_name,
    active_theme_source_path, bundled_theme_names, read_theme_source, set_theme_in_config,
    unset_theme_in_config,
};
use crate::VeilaError;

mod bundled_defaults_tests;
mod defaults_tests;
mod file_loading_tests;
mod nested_visual_fixture;
mod nested_visual_tests;
mod parsing_tests;
mod theme_loading_tests;
mod theme_mutation_tests;
