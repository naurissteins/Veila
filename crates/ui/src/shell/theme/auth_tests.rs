use veila_common::AppConfig;
use veila_renderer::{ClearColor, FrameSize, RenderScale, SoftwareBuffer};

use super::{ShellTheme, WidgetPositionTarget};
use crate::{ShellKey, ShellState};

#[test]
fn auth_positions_keep_sorted_backdrop_targets_when_scaled() {
    let config = AppConfig::from_toml_str(
        "[[visuals.backdrop]]\nname='late'\nz=5\n\
         [[visuals.backdrop]]\nname='disabled'\nenabled=false\n\
         [[visuals.backdrop]]\nname='early'\nz=-1\n\
         [visuals.input]\nrelative_to='early'\nx=3\ny=-3\n\
         [visuals.status]\nrelative_to='late'\nx=-7\ny=9\n",
    )
    .unwrap();
    let theme = ShellTheme::from_config(&config);
    let scaled = theme.scaled_for_render_at(RenderScale::from_units(150));
    let input = scaled.input.position.unwrap();
    let status = scaled.status.position.unwrap();

    assert_eq!(
        (input.target, input.x, input.y),
        (WidgetPositionTarget::Backdrop(0), 4, -4)
    );
    assert_eq!(
        (status.target, status.x, status.y),
        (WidgetPositionTarget::Backdrop(1), -9, 11)
    );
    assert_eq!(theme.input.position.unwrap().x, 3);
    assert_eq!(theme.status.position.unwrap().y, 9);
}

#[test]
fn auth_fractional_scaling_preserves_zero_and_optional_styles() {
    let config = AppConfig::from_toml_str(
        "[visuals.input]\nwidth=321\nheight=55\nradius=7\nborder_width=0\n\
         font_size=19\nbackground_color='rgba(20, 30, 40, 0.5)'\n\
         [visuals.reveal]\nfont_size=21\ntext='Unlock Ω'\n\
         [visuals.status]\nmode='external'\ncolor='#abcdef'\n",
    )
    .unwrap();
    let theme = ShellTheme::from_config(&config);
    let scaled = theme.scaled_for_render_at(RenderScale::from_units(150));

    assert_eq!(
        (scaled.input.width, scaled.input.height, scaled.input.radius),
        (Some(401), Some(69), 9)
    );
    assert_eq!(scaled.input.border_width, Some(0));
    assert_eq!(
        (scaled.input.font_size, scaled.reveal.font_size),
        (Some(24), Some(26))
    );
    assert_eq!(scaled.input.background_color, theme.input.background_color);
    assert_eq!(scaled.input.font_family, theme.input.font_family);
    assert_eq!(scaled.input.position, None);
    assert_eq!(scaled.reveal.text, "Unlock Ω");
    assert_eq!(scaled.status, theme.status);
    assert_eq!(scaled.placeholder, theme.placeholder);
    assert_eq!(scaled.eye, theme.eye);
    assert_eq!(scaled.caps_lock, theme.caps_lock);
}

#[test]
fn reloading_auth_styles_matches_fresh_hidden_and_challenge_frames() {
    let initial =
        AppConfig::from_toml_str("[visuals.input]\nreveal_on_interaction=true\n").unwrap();
    let updated = AppConfig::from_toml_str(
        "[visuals.input]\nreveal_on_interaction=true\nfont_size=23\nfont_weight=600\n\
         radius=7\nborder_width=3\nmask_color='#aabbcc'\n\
         [visuals.reveal]\ntext='Updated Ω'\nfont_size=19\nfont_style='italic'\n\
         [visuals.placeholder]\ncolor='#abcdef'\n\
         [visuals.eye]\nenabled=false\n\
         [visuals.caps_lock]\ncolor='#ccbb00'\n\
         [visuals.status]\nmode='external'\ncolor='#ffccaa'\nrejected_color='#cc0000'\n",
    )
    .unwrap();
    let mut reused = ShellState::new(ShellTheme::from_config(&initial), None, None, true);
    let theme = ShellTheme::from_config(&updated);
    for units in [120, 150, 180, 240] {
        let scale = RenderScale::from_units(units);
        let mut buffer = SoftwareBuffer::solid(
            scale.frame_size(FrameSize::new(640, 360)),
            ClearColor::opaque(0, 0, 0),
        )
        .unwrap();
        reused.render_at_scale(&mut buffer, scale);
    }
    reused.apply_theme(theme.clone(), None, None, true);
    let mut fresh = ShellState::new(theme, None, None, true);
    reused.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    fresh.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    for challenge in [false, true] {
        if challenge {
            for shell in [&mut reused, &mut fresh] {
                shell.handle_key(ShellKey::Character('x'));
                shell.authentication_challenge("Challenge Ω".into(), true);
            }
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
                "challenge={challenge}, units={units}"
            );
            assert_eq!(
                reused.widget_regions_at_scale(size, scale),
                fresh.widget_regions_at_scale(size, scale)
            );
        }
    }
}
