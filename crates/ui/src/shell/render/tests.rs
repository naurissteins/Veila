use super::{SceneTextInputs, ShellState, TextLayoutCache, layout::SceneMetrics};
use crate::shell::{ShellStatus, ShellTheme};
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    text::{TextStyle, bundled_clock_font_family},
};

#[test]
fn unfocused_input_style_uses_configured_input_border() {
    let mut shell = ShellState::default();
    shell.set_focus(false);
    let style = shell.input_style();

    assert_eq!(style.fill.alpha, 232);
    assert_eq!(
        style.border.expect("input border").color,
        shell.theme.input_border.with_alpha(210)
    );
}

#[test]
fn default_input_style_uses_input_border() {
    let shell = ShellState::default();
    let style = shell.input_style();

    assert_eq!(
        style.border.expect("default border").color,
        shell.theme.input_border.with_alpha(240)
    );
}

#[test]
fn focused_input_style_uses_input_border() {
    let mut shell = ShellState::default();
    shell.set_focus(true);
    let style = shell.input_style();

    assert_eq!(
        style.border.expect("focused border").color,
        shell.theme.input_border.with_alpha(240)
    );
}

#[test]
fn explicit_input_alpha_is_preserved() {
    let theme = ShellTheme {
        input: ClearColor::rgba(96, 164, 255, 51),
        input_border: ClearColor::rgba(96, 164, 255, 64),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.set_focus(false);
    let style = shell.input_style();

    assert_eq!(style.fill.alpha, 51);
    assert_eq!(style.border.expect("input border").color.alpha, 64);
}

#[test]
fn input_style_uses_configured_radius() {
    let theme = ShellTheme {
        input_radius: 18,
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.input_style();

    assert_eq!(style.radius, 18);
}

#[test]
fn input_style_uses_configured_border_width() {
    let theme = ShellTheme {
        input_border_width: Some(4),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.input_style();

    assert_eq!(style.border.expect("input border").thickness, 4);
}

#[test]
fn input_style_allows_disabling_border() {
    let theme = ShellTheme {
        input_border_width: Some(0),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.input_style();

    assert!(style.border.is_none());
}

#[test]
fn explicit_input_opacity_is_preserved_without_style_boost() {
    let theme = ShellTheme {
        input: ClearColor::rgba(255, 255, 255, 26),
        input_border: ClearColor::rgba(255, 255, 255, 31),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.set_focus(false);
    let style = shell.input_style();

    assert_eq!(style.fill.alpha, 26);
    assert_eq!(style.border.expect("input border").color.alpha, 31);
}

#[test]
fn avatar_style_uses_configured_placeholder_padding() {
    let theme = ShellTheme {
        avatar_placeholder_padding: Some(16),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(style.placeholder_padding, Some(16));
}

#[test]
fn avatar_style_uses_configured_icon_color() {
    let theme = ShellTheme {
        avatar_icon_color: Some(ClearColor::opaque(232, 238, 249)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(style.placeholder, ClearColor::rgba(232, 238, 249, 224));
}

#[test]
fn toggle_style_uses_configured_eye_icon_color() {
    let theme = ShellTheme {
        eye_icon_color: Some(ClearColor::opaque(244, 248, 255)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.toggle_style();

    assert_eq!(style.color, ClearColor::rgba(244, 248, 255, 184));
}

#[test]
fn toggle_style_scales_alpha_with_configured_eye_icon_opacity() {
    let theme = ShellTheme {
        eye_icon_color: Some(ClearColor::opaque(244, 248, 255)),
        eye_icon_opacity: Some(50),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.toggle_style();

    assert_eq!(style.color, ClearColor::rgba(244, 248, 255, 92));
}

#[test]
fn toggle_style_preserves_explicit_eye_icon_alpha_when_unset() {
    let theme = ShellTheme {
        eye_icon_color: Some(ClearColor::rgba(244, 248, 255, 128)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.toggle_style();

    assert_eq!(style.color.alpha, 92);
}

#[test]
fn mask_style_uses_configured_input_mask_color() {
    let theme = ShellTheme {
        input_mask_color: Some(ClearColor::opaque(169, 196, 255)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.mask_style();

    assert_eq!(style.bullet, ClearColor::opaque(169, 196, 255));
}

#[test]
fn avatar_style_uses_configured_ring_width() {
    let theme = ShellTheme {
        avatar_ring_width: Some(4),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(style.ring.expect("avatar ring").thickness, 4);
}

#[test]
fn avatar_style_uses_configured_ring_color() {
    let theme = ShellTheme {
        avatar_ring_color: Some(ClearColor::opaque(148, 178, 255)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(
        style.ring.expect("avatar ring").color,
        ClearColor::rgba(148, 178, 255, 108)
    );
}

#[test]
fn avatar_style_preserves_explicit_ring_alpha() {
    let theme = ShellTheme {
        avatar_ring_color: Some(ClearColor::rgba(148, 178, 255, 48)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(style.ring.expect("avatar ring").color.alpha, 48);
}

#[test]
fn avatar_style_allows_disabling_ring() {
    let theme = ShellTheme {
        avatar_ring_width: Some(0),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert!(style.ring.is_none());
}

#[test]
fn avatar_style_uses_configured_background_opacity() {
    let theme = ShellTheme {
        avatar_background: ClearColor::rgba(24, 30, 42, 255),
        avatar_background_opacity: Some(36),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(style.background.alpha, 92);
}

#[test]
fn avatar_style_preserves_explicit_panel_alpha_when_unset() {
    let theme = ShellTheme {
        avatar_background: ClearColor::rgba(24, 30, 42, 80),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(style.background.alpha, 80);
}

#[test]
fn scene_metrics_use_configured_avatar_size() {
    let theme = ShellTheme {
        avatar_size: Some(88),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let mut buffer = SoftwareBuffer::new(FrameSize::new(1280, 720)).expect("buffer");

    shell.render_overlay(&mut buffer);

    let metrics = SceneMetrics::from_frame(
        1280,
        720,
        shell.theme.input_width,
        shell.theme.input_height,
        shell.theme.avatar_size,
    );
    assert_eq!(metrics.avatar_size, 88);
}

#[test]
fn username_style_uses_configured_opacity_and_size() {
    let theme = ShellTheme {
        foreground: ClearColor::rgba(240, 244, 250, 255),
        username_opacity: Some(72),
        username_size: Some(3),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.username_text_style();

    assert_eq!(style.color.alpha, 184);
    assert_eq!(style.scale, 3);
}

#[test]
fn username_style_uses_configured_color() {
    let theme = ShellTheme {
        username_color: Some(ClearColor::opaque(215, 227, 255)),
        username_opacity: Some(72),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.username_text_style();

    assert_eq!(style.color.red, 215);
    assert_eq!(style.color.green, 227);
    assert_eq!(style.color.blue, 255);
    assert_eq!(style.color.alpha, 184);
}

#[test]
fn username_style_preserves_explicit_foreground_alpha_when_unset() {
    let theme = ShellTheme {
        foreground: ClearColor::rgba(240, 244, 250, 90),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.username_text_style();

    assert_eq!(style.color.alpha, 90);
    assert_eq!(style.scale, 2);
}

#[test]
fn clock_style_uses_configured_opacity() {
    let theme = ShellTheme {
        foreground: ClearColor::rgba(240, 244, 250, 255),
        clock_opacity: Some(96),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

    assert_eq!(style.color.alpha, 245);
    assert_eq!(style.scale, 5);
}

#[test]
fn clock_style_uses_configured_color() {
    let theme = ShellTheme {
        clock_color: Some(ClearColor::opaque(248, 251, 255)),
        clock_opacity: Some(96),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

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
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

    assert!(
        style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains(&bundled_family))
    );
}

#[test]
fn clock_style_defaults_to_bundled_font_family() {
    let shell = ShellState::default();
    let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

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
    assert_eq!(style.font_weight, None);
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
    let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

    assert_eq!(style.scale, 4);
}

#[test]
fn header_styles_do_not_add_extra_line_spacing() {
    let shell = ShellState::default();
    let clock_style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));
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
    let style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));

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
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let clock_style = shell.clock_text_style(SceneMetrics::from_frame(1280, 720, None, None, None));
    let date_style = shell.date_text_style();

    assert_eq!(clock_style.color.alpha, 90);
    assert_eq!(date_style.color.alpha, 90);
}

#[test]
fn scene_metrics_use_configured_input_dimensions() {
    let theme = ShellTheme {
        input_width: Some(280),
        input_height: Some(54),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let metrics = SceneMetrics::from_frame(
        1280,
        720,
        shell.theme.input_width,
        shell.theme.input_height,
        shell.theme.avatar_size,
    );

    assert_eq!(metrics.input_width, 280);
    assert_eq!(metrics.input_height, 54);
}

#[test]
fn placeholder_style_uses_configured_opacity() {
    let theme = ShellTheme {
        muted: ClearColor::rgba(72, 82, 108, 255),
        placeholder_opacity: Some(60),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.placeholder_text_style();

    assert_eq!(style.color.alpha, 153);
    assert_eq!(style.scale, 2);
}

#[test]
fn placeholder_style_uses_configured_color() {
    let theme = ShellTheme {
        placeholder_color: Some(ClearColor::opaque(134, 148, 180)),
        placeholder_opacity: Some(60),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.placeholder_text_style();

    assert_eq!(style.color.red, 134);
    assert_eq!(style.color.green, 148);
    assert_eq!(style.color.blue, 180);
    assert_eq!(style.color.alpha, 153);
}

#[test]
fn status_style_uses_configured_opacity() {
    let theme = ShellTheme {
        input_border: ClearColor::rgba(255, 255, 255, 255),
        status_opacity: Some(88),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.status_text_style();

    assert_eq!(style.color.alpha, 224);
    assert_eq!(style.scale, 2);
}

#[test]
fn status_style_uses_configured_color() {
    let theme = ShellTheme {
        status_color: Some(ClearColor::opaque(255, 224, 160)),
        status_opacity: Some(88),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.status_text_style();

    assert_eq!(style.color.red, 255);
    assert_eq!(style.color.green, 224);
    assert_eq!(style.color.blue, 160);
    assert_eq!(style.color.alpha, 224);
}

#[test]
fn placeholder_style_preserves_explicit_muted_alpha_when_unset() {
    let theme = ShellTheme {
        muted: ClearColor::rgba(72, 82, 108, 90),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.placeholder_text_style();

    assert_eq!(style.color.alpha, 90);
}

#[test]
fn status_style_preserves_explicit_pending_alpha_when_unset() {
    let theme = ShellTheme {
        pending: ClearColor::rgba(255, 194, 92, 90),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.status = ShellStatus::Pending;
    let style = shell.status_text_style();

    assert_eq!(style.color.alpha, 90);
}

#[test]
fn text_layout_cache_reuses_matching_clock_layout() {
    let mut cache = TextLayoutCache::default();
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);
    let style = TextStyle::new(ClearColor::opaque(255, 255, 255), 5);

    let first = cache.scene_text_blocks(SceneTextInputs {
        clock_text: "09:41",
        clock_style: style.clone(),
        date_text: "Tuesday",
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: Some("ramces"),
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: "Type your password to unlock",
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        metrics,
    });
    let cached_clock = cache.clock.block.clone().expect("cached clock block");
    let second = cache.scene_text_blocks(SceneTextInputs {
        clock_text: "09:41",
        clock_style: style,
        date_text: "Tuesday",
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: Some("ramces"),
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: "Type your password to unlock",
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        metrics,
    });

    assert_eq!(first.clock, second.clock);
    assert_eq!(cached_clock, second.clock);
}

#[test]
fn text_layout_cache_refreshes_when_clock_text_changes() {
    let mut cache = TextLayoutCache::default();
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    let first = cache.scene_text_blocks(SceneTextInputs {
        clock_text: "09:41",
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        date_text: "Tuesday",
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: None,
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: "Type your password to unlock",
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        metrics,
    });
    let second = cache.scene_text_blocks(SceneTextInputs {
        clock_text: "09:42",
        clock_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 5),
        date_text: "Tuesday",
        date_style: TextStyle::new(ClearColor::opaque(255, 255, 255), 2),
        username_text: None,
        username_style: TextStyle::new(ClearColor::opaque(240, 244, 250), 2),
        placeholder_text: "Type your password to unlock",
        placeholder_style: TextStyle::new(ClearColor::opaque(72, 82, 108), 2),
        status_text: None,
        status_style: TextStyle::new(ClearColor::opaque(255, 194, 92), 2),
        metrics,
    });

    assert_ne!(first.clock.lines, second.clock.lines);
    assert_eq!(
        cache.clock.key.as_ref().map(|key| key.text.as_str()),
        Some("09:42")
    );
}

#[test]
fn text_layout_cache_reuses_matching_revealed_secret_layout() {
    let mut cache = TextLayoutCache::default();
    let style = TextStyle::new(ClearColor::rgba(240, 244, 250, 236), 2);

    let first = cache.revealed_secret_block("secret", style.clone(), 212);
    let cached = cache
        .revealed_secret
        .block
        .clone()
        .expect("cached revealed secret block");
    let second = cache.revealed_secret_block("secret", style, 212);

    assert_eq!(first, second);
    assert_eq!(cached, second);
}
