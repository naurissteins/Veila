use veila_common::{AppConfig, ClockAlignment, FontStyle};
use veila_renderer::{ClearColor, FrameSize, RenderScale, SoftwareBuffer};

use super::{ShellTheme, WidgetPositionTarget};
use crate::ShellState;

#[test]
fn sparse_header_settings_preserve_distinct_font_fallbacks_and_zero_overrides() {
    let config = AppConfig::from_toml_str(
        "[visuals.clock]\nenabled=false\nfont_size=0\nmeridiem_font_size=0\nmeridiem_x=0\nmeridiem_y=0\n\
         [visuals.date]\nenabled=false\nfont_size=0\n",
    ).unwrap();
    let theme = ShellTheme::from_config(&config);
    assert!(!theme.clock.enabled && !theme.date.enabled);
    assert_eq!(theme.clock.font_family.as_deref(), Some("Geom"));
    assert_eq!(theme.clock.font_weight, Some(600));
    assert_eq!(theme.clock.font_style, Some(FontStyle::Normal));
    assert_eq!(
        (
            theme.date.font_family,
            theme.date.font_weight,
            theme.date.font_style
        ),
        (None, None, None)
    );
    assert_eq!(
        (
            theme.clock.font_size,
            theme.clock.meridiem_font_size,
            theme.date.font_size
        ),
        (Some(0), Some(0), Some(0))
    );
    assert_eq!(
        (theme.clock.meridiem_x, theme.clock.meridiem_y),
        (Some(0), Some(0))
    );
    assert_eq!(theme.clock.position, None);
    assert_eq!(theme.date.position, None);
}

#[test]
fn header_scaling_keeps_flow_and_meridiem_offsets_separate_from_backdrop_positions() {
    let config = AppConfig::from_toml_str(
        "[[visuals.backdrop]]\nname='header'\nz=5\n\
         [[visuals.backdrop]]\nname='earlier'\nz=-1\n\
         [visuals.clock]\nrelative_to='header'\nx=3\ny=-7\nfont_size=83\n\
         meridiem_font_size=19\nmeridiem_x=-7\nmeridiem_y=9\n\
         [visuals.date]\nrelative_to='earlier'\nx=-5\ny=9\nfont_size=19\n",
    )
    .unwrap();
    let mut theme = ShellTheme::from_config(&config);
    theme.clock.alignment = ClockAlignment::TopLeft;
    theme.clock.center_in_layer = true;
    theme.clock.offset_x = Some(-3);
    theme.clock.offset_y = Some(5);
    theme.clock.gap = Some(17);
    let scaled = theme.scaled_for_render_at(RenderScale::from_units(150));
    let clock = scaled.clock.position.unwrap();
    let date = scaled.date.position.unwrap();
    assert_eq!(
        (clock.target, clock.x, clock.y),
        (WidgetPositionTarget::Backdrop(1), 4, -9)
    );
    assert_eq!(
        (date.target, date.x, date.y),
        (WidgetPositionTarget::Backdrop(0), -6, 11)
    );
    assert_eq!(
        (
            scaled.clock.font_size,
            scaled.clock.meridiem_font_size,
            scaled.date.font_size
        ),
        (Some(104), Some(24), Some(24))
    );
    assert_eq!(
        (
            scaled.clock.offset_x,
            scaled.clock.offset_y,
            scaled.clock.gap
        ),
        (Some(-4), Some(6), Some(21))
    );
    assert_eq!(
        (scaled.clock.meridiem_x, scaled.clock.meridiem_y),
        (Some(-9), Some(11))
    );
    assert_eq!(scaled.clock.alignment, theme.clock.alignment);
    assert!(scaled.clock.center_in_layer);
    assert_eq!(scaled.clock.color, theme.clock.color);
    assert_eq!(theme.clock.offset_x, Some(-3));
}

#[test]
fn header_scaling_preserves_absent_overrides_and_non_pixel_styles() {
    let mut theme = ShellTheme::default();
    theme.clock.font_size = None;
    theme.clock.meridiem_font_size = None;
    theme.clock.meridiem_x = Some(0);
    theme.clock.meridiem_y = None;
    theme.clock.offset_x = None;
    theme.clock.offset_y = Some(0);
    theme.clock.gap = Some(0);
    theme.clock.position = None;
    theme.clock.color = None;
    theme.clock.font_family = None;
    theme.date.position = None;
    theme.date.font_size = None;
    theme.date.color = None;
    theme.date.font_family = Some("  Geom  ".into());
    let scaled = theme.scaled_for_render_at(RenderScale::from_units(180));
    assert_eq!(scaled.clock, theme.clock);
    assert_eq!(scaled.date, theme.date);
}

#[test]
fn header_reload_matches_fresh_frames_and_minute_deadlines_for_independent_visibility() {
    let initial = AppConfig::from_toml_str(
        "[visuals.clock]\nformat='12h'\nstyle='stacked'\n[visuals.date]\nformat='iso'\n",
    )
    .unwrap();
    for (clock_enabled, date_enabled) in
        [(true, true), (true, false), (false, true), (false, false)]
    {
        let updated = AppConfig::from_toml_str(&format!(
            "[visuals.clock]\nenabled={clock_enabled}\nformat='24h'\nfont_size=47\n\
             meridiem_font_size=17\nmeridiem_x=-3\nmeridiem_y=5\n\
             [visuals.date]\nenabled={date_enabled}\nformat='short'\nfont_size=23\n"
        ))
        .unwrap();
        let mut reused = ShellState::new(ShellTheme::from_config(&initial), None, None, true);
        let scale = RenderScale::from_units(150);
        let mut cached = SoftwareBuffer::solid(
            scale.frame_size(FrameSize::new(640, 360)),
            ClearColor::opaque(0, 0, 0),
        )
        .unwrap();
        reused.render_at_scale(&mut cached, scale);
        let theme = ShellTheme::from_config(&updated);
        reused.apply_theme(theme.clone(), None, None, true);
        let mut fresh = ShellState::new(theme, None, None, true);
        for shell in [&mut reused, &mut fresh] {
            shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
            assert_eq!(shell.clock.primary_text(shell.theme.clock.style), "00:00");
            assert_eq!(shell.clock.date_text(), "Thu, January 1");
            assert_eq!(
                shell.next_animation_in(std::time::Instant::now()).is_some(),
                clock_enabled || date_enabled
            );
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
                "clock={clock_enabled}, date={date_enabled}, units={units}"
            );
            assert_eq!(
                reused.widget_regions_at_scale(size, scale),
                fresh.widget_regions_at_scale(size, scale)
            );
        }
    }
}
