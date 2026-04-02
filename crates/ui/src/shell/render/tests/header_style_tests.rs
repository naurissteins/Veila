use super::*;

#[test]
fn clock_style_uses_configured_opacity() {
    let theme = ShellTheme {
        foreground: ClearColor::rgba(240, 244, 250, 255),
        clock_opacity: Some(96),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.clock_text_style(SceneMetrics::from_frame(
        1280,
        720,
        None,
        None,
        None,
        InputAlignment::CenterCenter,
    ));

    assert_eq!(style.color.alpha, 245);
    assert_eq!(style.scale, 14);
}

#[test]
fn clock_style_uses_configured_color() {
    let theme = ShellTheme {
        clock_color: Some(ClearColor::opaque(248, 251, 255)),
        clock_opacity: Some(96),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.clock_text_style(SceneMetrics::from_frame(
        1280,
        720,
        None,
        None,
        None,
        InputAlignment::CenterCenter,
    ));

    assert_eq!(style.color.red, 248);
    assert_eq!(style.color.green, 251);
    assert_eq!(style.color.blue, 255);
    assert_eq!(style.color.alpha, 245);
}

#[test]
fn clock_style_uses_configured_font_family() {
    let bundled_family =
        bundled_clock_font_family().expect("bundled clock font family should resolve");
    let theme = ShellTheme {
        clock_font_family: Some(bundled_family.clone()),
        clock_font_weight: Some(700),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.clock_text_style(SceneMetrics::from_frame(
        1280,
        720,
        None,
        None,
        None,
        InputAlignment::CenterCenter,
    ));

    assert!(
        style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains(&bundled_family))
    );
    assert_eq!(style.font_weight, Some(700));
}

#[test]
fn clock_style_defaults_to_bundled_font_family() {
    let shell = ShellState::default();
    let style = shell.clock_text_style(SceneMetrics::from_frame(
        1280,
        720,
        None,
        None,
        None,
        InputAlignment::CenterCenter,
    ));

    assert!(
        style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| {
                bundled_clock_font_family()
                    .as_ref()
                    .is_some_and(|family| debug.contains(family))
            })
    );
    assert_eq!(style.font_weight, Some(600));
}

#[test]
fn date_style_uses_configured_opacity() {
    let theme = ShellTheme {
        foreground: ClearColor::rgba(240, 244, 250, 255),
        date_opacity: Some(74),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.date_text_style();

    assert_eq!(style.color.alpha, 189);
    assert_eq!(style.scale, 2);
}

#[test]
fn date_style_uses_configured_color() {
    let theme = ShellTheme {
        date_color: Some(ClearColor::opaque(200, 212, 236)),
        date_opacity: Some(74),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.date_text_style();

    assert_eq!(style.color.red, 200);
    assert_eq!(style.color.green, 212);
    assert_eq!(style.color.blue, 236);
    assert_eq!(style.color.alpha, 189);
}

#[test]
fn clock_style_uses_configured_size() {
    let theme = ShellTheme {
        clock_size: Some(4),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.clock_text_style(SceneMetrics::from_frame(
        1280,
        720,
        None,
        None,
        None,
        InputAlignment::CenterCenter,
    ));

    assert_eq!(style.scale, 4);
}

#[test]
fn clock_meridiem_style_is_smaller_than_main_clock() {
    let theme = ShellTheme {
        clock_size: Some(12),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let metrics =
        SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);
    let clock_style = shell.clock_text_style(metrics);
    let meridiem_style = shell.clock_meridiem_text_style(metrics);

    assert!(meridiem_style.scale < clock_style.scale);
    assert_eq!(meridiem_style.line_spacing, 0);
}

#[test]
fn clock_meridiem_style_uses_configured_size() {
    let theme = ShellTheme {
        clock_meridiem_size: Some(5),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let metrics =
        SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);
    let meridiem_style = shell.clock_meridiem_text_style(metrics);

    assert_eq!(meridiem_style.scale, 5);
}

#[test]
fn header_styles_do_not_add_extra_line_spacing() {
    let shell = ShellState::default();
    let clock_style = shell.clock_text_style(SceneMetrics::from_frame(
        1280,
        720,
        None,
        None,
        None,
        InputAlignment::CenterCenter,
    ));
    let date_style = shell.date_text_style();

    assert_eq!(clock_style.line_spacing, 0);
    assert_eq!(date_style.line_spacing, 0);
}

#[test]
fn clock_style_allows_sizes_above_previous_cap() {
    let theme = ShellTheme {
        clock_size: Some(12),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.clock_text_style(SceneMetrics::from_frame(
        1280,
        720,
        None,
        None,
        None,
        InputAlignment::CenterCenter,
    ));

    assert_eq!(style.scale, 12);
}

#[test]
fn date_style_uses_configured_size() {
    let theme = ShellTheme {
        date_size: Some(3),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.date_text_style();

    assert_eq!(style.scale, 3);
}

#[test]
fn date_style_uses_configured_font_weight() {
    let theme = ShellTheme {
        date_font_weight: Some(600),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.date_text_style();

    assert_eq!(style.font_weight, Some(600));
}

#[test]
fn date_style_uses_configured_font_family() {
    let theme = ShellTheme {
        date_font_family: Some(String::from("Geom")),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.date_text_style();

    assert!(
        style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Geom"))
    );
}

#[test]
fn date_style_allows_sizes_above_previous_cap() {
    let theme = ShellTheme {
        date_size: Some(12),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.date_text_style();

    assert_eq!(style.scale, 12);
}

#[test]
fn header_styles_preserve_explicit_foreground_alpha_when_unset() {
    let theme = ShellTheme {
        foreground: ClearColor::rgba(240, 244, 250, 90),
        clock_color: None,
        clock_opacity: None,
        date_color: None,
        date_opacity: None,
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let clock_style = shell.clock_text_style(SceneMetrics::from_frame(
        1280,
        720,
        None,
        None,
        None,
        InputAlignment::CenterCenter,
    ));
    let date_style = shell.date_text_style();

    assert_eq!(clock_style.color.alpha, 90);
    assert_eq!(date_style.color.alpha, 90);
}
