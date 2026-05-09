use super::*;

#[test]
fn unfocused_input_style_uses_configured_input_border() {
    let mut shell = ShellState::default();
    shell.set_focus(false);
    let style = shell.input_style();

    assert_eq!(style.fill.alpha, 10);
    assert!(style.border.is_none());
}

#[test]
fn default_input_style_uses_input_border() {
    let shell = ShellState::default();
    let style = shell.input_style();

    assert!(style.border.is_none());
}

#[test]
fn focused_input_style_uses_input_border() {
    let mut shell = ShellState::new(ShellTheme::default(), None, None, true);
    shell.set_focus(true);
    let style = shell.input_style();

    assert!(style.border.is_none());
}

#[test]
fn explicit_input_alpha_is_preserved() {
    let theme = ShellTheme {
        input: ClearColor::rgba(96, 164, 255, 51),
        input_border: ClearColor::rgba(96, 164, 255, 64),
        input_border_width: Some(2),
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
fn rejected_input_style_uses_rejected_status_color_for_border() {
    let theme = ShellTheme {
        status_rejected_color: Some(ClearColor::opaque(220, 96, 96)),
        input_border_width: Some(2),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.status = ShellStatus::Rejected {
        retry_until: None,
        displayed_retry_seconds: None,
        failed_attempts: Some(1),
    };

    let style = shell.input_style();

    assert_eq!(
        style.border.expect("input border").color,
        ClearColor::rgba(220, 96, 96, 240)
    );
}

#[test]
fn selected_input_style_strengthens_fill_and_border() {
    let theme = ShellTheme {
        input: ClearColor::rgba(96, 164, 255, 51),
        input_border: ClearColor::rgba(96, 164, 255, 64),
        input_border_width: Some(2),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.handle_key(ShellKey::Character('a'));
    shell.handle_key(ShellKey::SelectAll);

    let style = shell.input_style();

    assert_eq!(style.fill.alpha, 88);
    assert_eq!(style.border.expect("input border").color.alpha, 148);
}

#[test]
fn explicit_input_opacity_is_preserved_without_style_boost() {
    let theme = ShellTheme {
        input: ClearColor::rgba(255, 255, 255, 26),
        input_border: ClearColor::rgba(255, 255, 255, 31),
        input_border_width: Some(2),
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
        eye_icon_color: Some(ClearColor::rgba(244, 248, 255, 184)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.toggle_style();

    assert_eq!(style.color, ClearColor::rgba(244, 248, 255, 132));
}

#[test]
fn toggle_style_uses_opaque_eye_fallback_alpha() {
    let theme = ShellTheme {
        eye_icon_color: Some(ClearColor::opaque(244, 248, 255)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.toggle_style();

    assert_eq!(style.color, ClearColor::rgba(244, 248, 255, 132));
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
        avatar_ring_width: Some(1),
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
        avatar_ring_width: Some(1),
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
fn avatar_style_preserves_explicit_background_alpha() {
    let theme = ShellTheme {
        avatar_background: ClearColor::rgba(24, 30, 42, 80),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(style.background.alpha, 80);
}

#[test]
fn avatar_style_uses_fallback_alpha_for_opaque_backgrounds() {
    let theme = ShellTheme {
        avatar_background: ClearColor::opaque(24, 30, 42),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.avatar_style();

    assert_eq!(style.background.alpha, 104);
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
fn username_style_uses_configured_alpha_and_size() {
    let theme = ShellTheme {
        username_color: Some(ClearColor::rgba(240, 244, 250, 184)),
        username_font_size: Some(22),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.username_text_style();

    assert_eq!(style.color.alpha, 184);
    assert_eq!(style.font_size_px, Some(22));
}

#[test]
fn username_style_uses_configured_font_family_and_weight() {
    let theme = ShellTheme {
        username_font_family: Some(String::from("Geom")),
        username_font_weight: Some(600),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.username_text_style();

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
fn username_style_uses_configured_color() {
    let theme = ShellTheme {
        username_color: Some(ClearColor::rgba(215, 227, 255, 184)),
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
        username_color: None,
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.username_text_style();

    assert_eq!(style.color.alpha, 90);
    assert_eq!(style.font_size_px, Some(28));
}

#[test]
fn placeholder_style_uses_fallback_alpha_for_opaque_muted_colors() {
    let theme = ShellTheme {
        muted: ClearColor::rgba(72, 82, 108, 255),
        placeholder_color: None,
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.placeholder_text_style();

    assert_eq!(style.color.alpha, 154);
    assert_eq!(style.font_size_px, Some(16));
}

#[test]
fn placeholder_style_uses_configured_color() {
    let theme = ShellTheme {
        placeholder_color: Some(ClearColor::rgba(134, 148, 180, 153)),
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
fn input_text_styles_use_configured_font_family_and_weight() {
    let theme = ShellTheme {
        input_font_family: Some(String::from("Geom")),
        input_font_weight: Some(600),
        input_font_size: Some(22),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let placeholder_style = shell.placeholder_text_style();
    let revealed_secret_style = shell.revealed_secret_text_style();

    assert!(
        placeholder_style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Geom"))
    );
    assert_eq!(placeholder_style.font_weight, Some(600));
    assert_eq!(placeholder_style.font_size_px, Some(22));
    assert!(
        revealed_secret_style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Geom"))
    );
    assert_eq!(revealed_secret_style.font_weight, Some(600));
    assert_eq!(revealed_secret_style.font_size_px, Some(22));
}

#[test]
fn status_style_uses_fallback_alpha_for_opaque_colors() {
    let theme = ShellTheme {
        input_border: ClearColor::rgba(255, 255, 255, 255),
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
        status_color: Some(ClearColor::rgba(255, 224, 160, 224)),
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
        placeholder_color: None,
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.placeholder_text_style();

    assert_eq!(style.color.alpha, 90);
}

#[test]
fn reveal_style_uses_configured_color_alpha_and_font() {
    let theme = ShellTheme {
        reveal_color: Some(ClearColor::rgba(214, 227, 255, 168)),
        reveal_font_family: Some(String::from("Geom")),
        reveal_font_weight: Some(500),
        reveal_font_style: Some(veila_common::FontStyle::Italic),
        reveal_font_size: Some(2),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.reveal_text_style();

    assert_eq!(style.color.red, 214);
    assert_eq!(style.color.green, 227);
    assert_eq!(style.color.blue, 255);
    assert_eq!(style.color.alpha, 168);
    assert_eq!(style.scale, 2);
    assert_eq!(style.font_weight, Some(500));
    assert_eq!(
        style.font_style,
        Some(veila_renderer::text::FontStyle::Italic)
    );
    assert!(
        style
            .font_family
            .as_ref()
            .map(|family| format!("{family:?}"))
            .is_some_and(|debug| debug.contains("Geom"))
    );
}

#[test]
fn reveal_style_falls_back_to_placeholder_style_defaults() {
    let shell = ShellState::default();

    assert_eq!(shell.reveal_text_style(), shell.placeholder_text_style());
}

#[test]
fn status_style_preserves_explicit_pending_alpha_when_unset() {
    let theme = ShellTheme {
        pending: ClearColor::rgba(255, 194, 92, 90),
        status_color: None,
        status_pending_color: None,
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.status = ShellStatus::Pending {
        started_at: std::time::Instant::now(),
        visible_after: std::time::Instant::now(),
        shown: true,
    };
    let style = shell.status_text_style();

    assert_eq!(style.color.alpha, 90);
}

#[test]
fn pending_status_style_prefers_state_specific_status_override() {
    let theme = ShellTheme {
        status_color: Some(ClearColor::opaque(255, 255, 255)),
        status_pending_color: Some(ClearColor::rgba(12, 34, 56, 153)),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.status = ShellStatus::Pending {
        started_at: std::time::Instant::now(),
        visible_after: std::time::Instant::now(),
        shown: true,
    };

    let style = shell.status_text_style();

    assert_eq!(style.color.red, 12);
    assert_eq!(style.color.green, 34);
    assert_eq!(style.color.blue, 56);
    assert_eq!(style.color.alpha, 153);
}

#[test]
fn rejected_status_style_prefers_state_specific_status_override() {
    let theme = ShellTheme {
        status_color: Some(ClearColor::opaque(255, 255, 255)),
        status_rejected_color: Some(ClearColor::rgba(180, 40, 40, 179)),
        ..ShellTheme::default()
    };
    let mut shell = ShellState::new(theme, None, None, true);
    shell.status = ShellStatus::Rejected {
        retry_until: None,
        displayed_retry_seconds: None,
        failed_attempts: None,
    };

    let style = shell.status_text_style();

    assert_eq!(style.color.red, 180);
    assert_eq!(style.color.green, 40);
    assert_eq!(style.color.blue, 40);
    assert_eq!(style.color.alpha, 179);
}

#[test]
fn caps_lock_icon_style_uses_dedicated_override() {
    let theme = ShellTheme {
        status_color: Some(ClearColor::rgba(255, 224, 160, 224)),
        caps_lock_color: Some(ClearColor::rgba(255, 211, 122, 163)),
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let style = shell.caps_lock_icon_style();

    assert_eq!(style.color.red, 255);
    assert_eq!(style.color.green, 211);
    assert_eq!(style.color.blue, 122);
    assert_eq!(style.color.alpha, 163);
}

#[test]
fn pending_status_text_stays_hidden_until_delay_elapses() {
    let mut shell = ShellState::default();

    let action = shell.handle_key(ShellKey::Character('a'));
    assert!(matches!(action, ShellAction::None));
    let action = shell.handle_key(ShellKey::Enter);
    assert_eq!(action, ShellAction::Submit(String::from("a")));

    assert_eq!(shell.status_text(), None);
}

#[test]
fn pending_status_text_appears_after_delay() {
    let mut shell = ShellState::default();

    let _ = shell.handle_key(ShellKey::Character('a'));
    let _ = shell.handle_key(ShellKey::Enter);
    std::thread::sleep(std::time::Duration::from_millis(1_050));

    assert!(shell.advance_animated_state());
    assert_eq!(
        shell.status_text().as_deref(),
        Some("Checking authentication")
    );
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
