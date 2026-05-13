use super::*;

#[test]
fn keyboard_layout_style_uses_configured_size() {
    let theme = ShellTheme {
        keyboard_size: Some(24),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.keyboard_layout_text_style();

    assert_eq!(style.font_size_px, Some(24));
    assert_eq!(style.scale, 1);
    assert_eq!(style.line_spacing, 0);
}

#[test]
fn keyboard_layout_style_uses_configured_color() {
    let theme = ShellTheme {
        keyboard_color: Some(ClearColor::rgba(232, 238, 249, 173)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.keyboard_layout_text_style();

    assert_eq!(style.color.red, 232);
    assert_eq!(style.color.green, 238);
    assert_eq!(style.color.blue, 249);
    assert_eq!(style.color.alpha, 173);
}

#[test]
fn keyboard_layout_style_defaults_to_geom() {
    let shell = ShellState::new(ShellTheme::default(), None, None, true);
    let style = shell.keyboard_layout_text_style();

    assert!(
        style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Geom"))
    );
    assert_eq!(style.font_weight, Some(600));
}

#[test]
fn weather_styles_use_configured_font_size_px() {
    let theme = ShellTheme {
        foreground: ClearColor::rgba(240, 244, 250, 255),
        muted: ClearColor::rgba(180, 190, 210, 255),
        weather_temperature_color: Some(ClearColor::rgba(255, 255, 255, 186)),
        weather_location_color: Some(ClearColor::rgba(214, 227, 255, 74)),
        weather_temperature_font_family: Some(String::from("Prototype")),
        weather_temperature_font_weight: Some(600),
        weather_temperature_letter_spacing: Some(2),
        weather_location_font_family: Some(String::from("Geom")),
        weather_location_font_weight: Some(500),
        weather_temperature_font_size: Some(42),
        weather_location_font_size: Some(22),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let temperature_style = shell.weather_temperature_text_style();
    let location_style = shell.weather_location_text_style();

    assert_eq!(temperature_style.font_size_px, Some(42));
    assert_eq!(location_style.font_size_px, Some(22));
    assert_eq!(temperature_style.color.alpha, 186);
    assert_eq!(location_style.color.alpha, 74);
    assert_eq!(temperature_style.color.red, 255);
    assert_eq!(location_style.color.red, 214);
    assert_eq!(temperature_style.line_spacing, 0);
    assert_eq!(temperature_style.letter_spacing, 2);
    assert_eq!(location_style.line_spacing, 0);
    assert_eq!(temperature_style.font_weight, Some(600));
    assert_eq!(location_style.font_weight, Some(500));
    assert!(
        temperature_style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Prototype"))
    );
    assert!(
        location_style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Geom"))
    );
}

#[test]
fn weather_styles_preserve_opaque_configured_colors() {
    let theme = ShellTheme {
        weather_temperature_color: Some(ClearColor::opaque(249, 226, 175)),
        weather_location_color: Some(ClearColor::opaque(249, 226, 175)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);

    assert_eq!(
        shell.weather_temperature_text_style().color,
        ClearColor::opaque(249, 226, 175)
    );
    assert_eq!(
        shell.weather_location_text_style().color,
        ClearColor::opaque(249, 226, 175)
    );
}

#[test]
fn now_playing_styles_use_configured_theme_values() {
    let theme = ShellTheme {
        now_playing_title_color: Some(ClearColor::rgba(248, 251, 255, 208)),
        now_playing_artist_color: Some(ClearColor::rgba(200, 212, 236, 99)),
        now_playing_title_font_family: Some("Geom".to_owned()),
        now_playing_artist_font_family: Some("Prototype".to_owned()),
        now_playing_title_font_weight: Some(700),
        now_playing_artist_font_weight: Some(500),
        now_playing_title_font_size: Some(22),
        now_playing_artist_font_size: Some(16),
        now_playing_title_width: Some(220),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let title_style = shell.now_playing_title_text_style();
    let artist_style = shell.now_playing_artist_text_style();

    assert_eq!(title_style.color, ClearColor::rgba(248, 251, 255, 208));
    assert_eq!(title_style.scale, 1);
    assert_eq!(title_style.font_size_px, Some(22));
    assert_eq!(title_style.font_weight, Some(700));
    assert!(
        title_style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Geom"))
    );
    assert_eq!(artist_style.color, ClearColor::rgba(200, 212, 236, 99));
    assert_eq!(artist_style.scale, 1);
    assert_eq!(artist_style.font_size_px, Some(16));
    assert_eq!(artist_style.font_weight, Some(500));
    assert_eq!(shell.theme.now_playing_title_width, Some(220));
    assert!(
        artist_style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Prototype"))
    );
}

#[test]
fn now_playing_blocks_stay_single_line_and_truncate() {
    let mut cache = TextLayoutCache::default();
    let title = cache.now_playing_title_block(
        "An extremely long track title that should not wrap to a second line",
        TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        120,
    );
    let artist = cache.now_playing_artist_block(
        "A very long artist name that should also truncate",
        TextStyle::new(ClearColor::opaque(200, 212, 236), 1),
        100,
    );

    assert_eq!(title.lines.len(), 1);
    assert_eq!(artist.lines.len(), 1);
    assert!(title.width <= 120);
    assert!(artist.width <= 100);
    assert!(title.lines[0].ends_with("..."));
    assert!(artist.lines[0].ends_with("..."));
}
