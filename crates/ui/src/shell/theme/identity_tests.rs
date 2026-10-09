use veila_common::{AppConfig, HorizontalAlign, VerticalAlign};
use veila_renderer::{ClearColor, RenderScale};

use super::{ShellTheme, WidgetPosition, WidgetPositionTarget};

#[test]
fn sparse_identity_settings_keep_fallbacks_and_explicit_zero_values() {
    let config = AppConfig::from_toml_str(
        "[visuals.avatar]\nenabled=false\nradius=65535\nring_width=0\n\
         placeholder_padding=0\n[visuals.username]\nenabled=false\nfont_size=0\n",
    )
    .unwrap();
    let theme = ShellTheme::from_config(&config);

    assert!(!theme.avatar.enabled && !theme.username.enabled);
    assert_eq!(
        theme.avatar.background_color,
        ClearColor::opaque(22, 28, 38)
    );
    assert_eq!(theme.avatar.radius, Some(320));
    assert_eq!(theme.avatar.ring_width, Some(0));
    assert_eq!(theme.avatar.placeholder_padding, Some(0));
    assert_eq!(theme.avatar.size, Some(150));
    assert_eq!(theme.username.font_family, None);
    assert_eq!(theme.username.font_size, Some(0));
    assert_eq!(theme.avatar.position, None);
    assert_eq!(theme.username.position, None);
}

#[test]
fn identity_scaling_keeps_flow_offsets_separate_from_backdrop_positions() {
    let config = AppConfig::from_toml_str(
        "[[visuals.backdrop]]\nname='identity'\nz=5\n\
         [[visuals.backdrop]]\nname='earlier'\nz=-1\n\
         [visuals.avatar]\nrelative_to='identity'\nx=3\ny=-7\nsize=83\nradius=17\n\
         placeholder_padding=9\nring_width=3\n\
         [visuals.username]\nrelative_to='earlier'\nx=-5\ny=9\nfont_size=19\n",
    )
    .unwrap();
    let mut theme = ShellTheme::from_config(&config);
    theme.avatar.offset_y = Some(-3);
    theme.avatar.gap = Some(17);
    theme.username.offset_y = Some(5);
    theme.username.gap = Some(23);
    let scaled = theme.scaled_for_render_at(RenderScale::from_units(150));
    let avatar = scaled.avatar.position.unwrap();
    let username = scaled.username.position.unwrap();

    assert_eq!(
        (avatar.target, avatar.x, avatar.y),
        (WidgetPositionTarget::Backdrop(1), 4, -9)
    );
    assert_eq!(
        (username.target, username.x, username.y),
        (WidgetPositionTarget::Backdrop(0), -6, 11)
    );
    assert_eq!(
        (
            scaled.avatar.size,
            scaled.avatar.radius,
            scaled.avatar.placeholder_padding,
            scaled.avatar.ring_width
        ),
        (Some(104), Some(21), Some(11), Some(4))
    );
    assert_eq!(
        (scaled.avatar.offset_y, scaled.avatar.gap),
        (Some(-4), Some(21))
    );
    assert_eq!(
        (
            scaled.username.font_size,
            scaled.username.offset_y,
            scaled.username.gap
        ),
        (Some(24), Some(6), Some(29))
    );
    assert_eq!(theme.avatar.offset_y, Some(-3));
    assert_eq!(theme.username.offset_y, Some(5));
}

#[test]
fn identity_scaling_preserves_absent_overrides_and_non_pixel_styles() {
    let mut theme = ShellTheme::default();
    theme.avatar.size = None;
    theme.avatar.radius = Some(0);
    theme.avatar.offset_y = None;
    theme.avatar.position = None;
    theme.avatar.placeholder_padding = None;
    theme.avatar.gap = Some(0);
    theme.avatar.ring_color = None;
    theme.avatar.ring_width = Some(0);
    theme.username.font_family = Some("  Geom  ".into());
    theme.username.font_size = None;
    theme.username.offset_y = None;
    theme.username.gap = Some(0);
    theme.username.color = None;
    theme.username.position = Some(WidgetPosition {
        halign: HorizontalAlign::Right,
        valign: VerticalAlign::Bottom,
        x: -3,
        y: 5,
        target: WidgetPositionTarget::Screen,
    });
    let scaled = theme.scaled_for_render_at(RenderScale::from_units(180));

    assert_eq!(scaled.avatar, theme.avatar);
    assert_eq!(scaled.username.font_family, theme.username.font_family);
    assert_eq!(scaled.username.font_size, None);
    assert_eq!(scaled.username.color, None);
    assert_eq!(scaled.username.offset_y, None);
    assert_eq!(scaled.username.gap, Some(0));
    assert_eq!(
        scaled
            .username
            .position
            .map(|position| (position.x, position.y)),
        Some((-5, 8))
    );
    assert_eq!(scaled.username.font_weight, theme.username.font_weight);
    assert_eq!(scaled.username.font_style, theme.username.font_style);
}
