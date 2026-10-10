use veila_common::{
    AppConfig, FontStyle, HorizontalAlign, VerticalAlign, WeatherCondition, WeatherSnapshot,
    WeatherUnit,
};
use veila_renderer::{ClearColor, FrameSize, RenderScale, SoftwareBuffer};

use super::{ShellTheme, WidgetPositionTarget};
use crate::ShellState;

#[test]
fn sparse_weather_preserves_absent_overrides_and_explicit_zero_values() {
    let config = AppConfig::from_toml_str(
        "[visuals.weather.icon]\nenabled=false\nsize=0\nopacity=0\n\
         [visuals.weather.temperature]\nfont_size=0\nletter_spacing=0\n\
         [visuals.weather.location]\nfont_size=0\n",
    )
    .unwrap();
    let weather = ShellTheme::from_config(&config).weather;
    assert!(weather.enabled && !weather.icon_enabled);
    assert!(weather.temperature_enabled && weather.location_enabled);
    assert_eq!(
        (weather.icon_size, weather.icon_opacity),
        (Some(0), Some(0))
    );
    assert_eq!(
        (
            weather.temperature_font_size,
            weather.location_font_size,
            weather.temperature_letter_spacing
        ),
        (Some(0), Some(0), Some(0))
    );
    assert_eq!(
        (
            weather.temperature_font_family,
            weather.location_font_family
        ),
        (None, None)
    );
    assert_eq!(
        (
            weather.temperature_font_weight,
            weather.location_font_weight
        ),
        (None, None)
    );
    assert_eq!(
        (weather.temperature_font_style, weather.location_font_style),
        (None, None)
    );
    assert_eq!(
        (weather.temperature_color, weather.location_color),
        (None, None)
    );
    assert!(
        weather.icon_position.is_none()
            && weather.temperature_position.is_none()
            && weather.location_position.is_none()
    );
    // Overall visibility is derived from the three part toggles.
    let hidden = AppConfig::from_toml_str(
        "[visuals.weather.icon]\nenabled=false\n\
         [visuals.weather.temperature]\nenabled=false\n\
         [visuals.weather.location]\nenabled=false\n",
    )
    .unwrap();
    assert!(!ShellTheme::from_config(&hidden).weather.enabled);
}

#[test]
fn weather_scaling_preserves_styles_and_sorted_backdrop_position_defaults() {
    let config = AppConfig::from_toml_str(
        "[[visuals.backdrop]]\nname='weather'\nz=3\n\
         [[visuals.backdrop]]\nname='earlier'\nz=-1\n\
         [visuals.weather.icon]\nrelative_to='weather'\nx=-7\ny=9\nsize=53\nopacity=63\n\
         [visuals.weather.temperature]\nrelative_to='earlier'\nx=3\ny=-5\nfont_size=29\nletter_spacing=3\nfont_weight=500\ncolor='#AABBCC'\n\
         [visuals.weather.location]\nrelative_to='missing'\nx=-3\ny=7\nfont_size=17\nfont_style='italic'\n",
    ).unwrap();
    let theme = ShellTheme::from_config(&config);
    let weather = theme
        .scaled_for_render_at(RenderScale::from_units(150))
        .weather;
    let icon = weather.icon_position.unwrap();
    let temperature = weather.temperature_position.unwrap();
    let location = weather.location_position.unwrap();
    assert_eq!(
        (icon.target, icon.x, icon.y),
        (WidgetPositionTarget::Backdrop(1), -9, 11)
    );
    assert_eq!(
        (temperature.target, temperature.x, temperature.y),
        (WidgetPositionTarget::Backdrop(0), 4, -6)
    );
    assert_eq!(
        (location.target, location.x, location.y),
        (WidgetPositionTarget::Screen, -4, 9)
    );
    for position in [icon, temperature, location] {
        assert_eq!(
            (position.halign, position.valign),
            (HorizontalAlign::Left, VerticalAlign::Bottom)
        );
    }
    assert_eq!(
        (
            weather.icon_size,
            weather.temperature_font_size,
            weather.location_font_size,
            weather.temperature_letter_spacing
        ),
        (Some(66), Some(36), Some(21), Some(4))
    );
    assert_eq!(weather.icon_opacity, Some(63));
    assert_eq!(weather.temperature_font_weight, Some(500));
    assert_eq!(weather.location_font_style, Some(FontStyle::Italic));
    assert_eq!(weather.temperature_color, theme.weather.temperature_color);
    assert_eq!(theme.weather.icon_size, Some(53));
}

#[test]
fn weather_scaling_preserves_absent_dimensions_and_non_pixel_overrides() {
    let mut theme = ShellTheme::default();
    let weather = &mut theme.weather;
    weather.icon_position = None;
    weather.temperature_position = None;
    weather.location_position = None;
    weather.icon_size = None;
    weather.temperature_font_size = None;
    weather.location_font_size = Some(0);
    weather.temperature_letter_spacing = Some(0);
    weather.temperature_font_family = Some("  Geom  ".into());
    weather.location_font_family = Some("sans-serif".into());
    weather.icon_opacity = Some(41);
    assert_eq!(
        theme
            .scaled_for_render_at(RenderScale::from_units(180))
            .weather,
        theme.weather
    );
    assert!(
        theme
            .font_warmup_families()
            .iter()
            .any(|family| family == "Geom")
    );
    assert!(
        theme
            .font_warmup_families()
            .iter()
            .any(|family| family == "sans-serif")
    );
}

#[test]
fn weather_reload_matches_fresh_pixels_and_geometry_with_visibility_and_missing_data() {
    let initial = AppConfig::from_toml_str("[visuals.weather.icon]\nx=-7\nsize=53\n").unwrap();
    let updated = AppConfig::from_toml_str(
        "[visuals.weather.icon]\nx=-13\nsize=0\nopacity=63\n\
         [visuals.weather.temperature]\ny=-91\nfont_size=37\nletter_spacing=3\ncolor='#AABBCC'\n\
         [visuals.weather.location]\ny=-53\nfont_size=21\nfont_weight=700\n",
    )
    .unwrap();
    for mask in 0..16 {
        for available in [false, true] {
            let mut theme = ShellTheme::from_config(&updated);
            theme.weather.enabled = mask & 8 != 0;
            theme.weather.icon_enabled = mask & 1 != 0;
            theme.weather.temperature_enabled = mask & 2 != 0;
            theme.weather.location_enabled = mask & 4 != 0;
            let snapshot = available.then_some(WeatherSnapshot {
                temperature_celsius: -17,
                condition: WeatherCondition::Snow,
                fetched_at_unix: 1,
            });
            let mut reused = ShellState::new(ShellTheme::from_config(&initial), None, None, false);
            let mut cached =
                SoftwareBuffer::solid(FrameSize::new(640, 360), ClearColor::opaque(0, 0, 0))
                    .unwrap();
            reused.render(&mut cached);
            reused.apply_theme_with_username_and_weather(
                theme.clone(),
                None,
                None,
                None,
                false,
                Some(" Rīga Ω ".into()),
                snapshot.clone(),
                WeatherUnit::Fahrenheit,
                None,
                None,
            );
            let mut fresh = ShellState::new_with_username_and_weather(
                theme,
                None,
                None,
                None,
                false,
                Some(" Rīga Ω ".into()),
                snapshot,
                WeatherUnit::Fahrenheit,
                None,
            );
            for shell in [&mut reused, &mut fresh] {
                shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
            }
            for units in [120, 150, 180, 240] {
                let scale = RenderScale::from_units(units);
                let size = scale.frame_size(FrameSize::new(640, 360));
                let mut actual = SoftwareBuffer::solid(size, ClearColor::opaque(0, 0, 0)).unwrap();
                let mut expected = actual.clone();
                reused.render_at_scale(&mut actual, scale);
                fresh.render_at_scale(&mut expected, scale);
                assert_eq!(
                    actual.pixels(),
                    expected.pixels(),
                    "mask={mask}, available={available}, units={units}"
                );
                assert_eq!(
                    reused.widget_regions_at_scale(size, scale),
                    fresh.widget_regions_at_scale(size, scale)
                );
            }
        }
    }
}
