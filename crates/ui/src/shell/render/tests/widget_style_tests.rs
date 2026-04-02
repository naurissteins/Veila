use super::*;

#[test]
fn keyboard_layout_style_uses_configured_size() {
    let theme = ShellTheme {
        keyboard_size: Some(3),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.keyboard_layout_text_style();

    assert_eq!(style.scale, 3);
    assert_eq!(style.line_spacing, 0);
}

#[test]
fn keyboard_layout_style_uses_configured_color_and_opacity() {
    let theme = ShellTheme {
        keyboard_color: Some(ClearColor::opaque(232, 238, 249)),
        keyboard_opacity: Some(68),
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
fn weather_styles_use_configured_widget_size() {
    let theme = ShellTheme {
        foreground: ClearColor::rgba(240, 244, 250, 255),
        muted: ClearColor::rgba(180, 190, 210, 255),
        weather_opacity: Some(50),
        weather_temperature_opacity: Some(80),
        weather_location_opacity: Some(40),
        weather_temperature_color: Some(ClearColor::opaque(255, 255, 255)),
        weather_location_color: Some(ClearColor::opaque(214, 227, 255)),
        weather_size: Some(4),
        weather_temperature_font_family: Some(String::from("Prototype")),
        weather_temperature_font_weight: Some(600),
        weather_temperature_letter_spacing: Some(2),
        weather_location_font_family: Some(String::from("Geom")),
        weather_location_font_weight: Some(500),
        weather_temperature_size: Some(12),
        weather_location_size: Some(2),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let temperature_style = shell.weather_temperature_text_style();
    let location_style = shell.weather_location_text_style();

    assert_eq!(temperature_style.scale, 12);
    assert_eq!(location_style.scale, 2);
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
fn now_playing_styles_use_configured_theme_values() {
    let theme = ShellTheme {
        now_playing_title_color: Some(ClearColor::opaque(248, 251, 255)),
        now_playing_artist_color: Some(ClearColor::opaque(200, 212, 236)),
        now_playing_title_font_family: Some("Geom".to_owned()),
        now_playing_artist_font_family: Some("Prototype".to_owned()),
        now_playing_title_font_weight: Some(700),
        now_playing_artist_font_weight: Some(500),
        now_playing_title_opacity: Some(88),
        now_playing_artist_opacity: Some(54),
        now_playing_title_size: Some(3),
        now_playing_artist_size: Some(2),
        now_playing_content_gap: Some(18),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let title_style = shell.now_playing_title_text_style();
    let artist_style = shell.now_playing_artist_text_style();

    assert_eq!(title_style.color, ClearColor::rgba(248, 251, 255, 208));
    assert_eq!(title_style.scale, 3);
    assert_eq!(title_style.font_weight, Some(700));
    assert!(
        title_style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Geom"))
    );
    assert_eq!(artist_style.color, ClearColor::rgba(200, 212, 236, 99));
    assert_eq!(artist_style.scale, 2);
    assert_eq!(artist_style.font_weight, Some(500));
    assert_eq!(shell.theme.now_playing_content_gap, Some(18));
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
