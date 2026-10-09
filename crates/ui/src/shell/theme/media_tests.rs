use std::time::{Duration, Instant};

use veila_common::{AppConfig, NowPlayingSnapshot};
use veila_renderer::{ClearColor, FrameSize, RenderScale, SoftwareBuffer};

use super::{ShellTheme, WidgetPositionTarget};
use crate::{ShellAnimationUpdate, ShellState, WidgetKind};

#[test]
fn sparse_media_settings_preserve_absent_overrides_and_explicit_zero_values() {
    let config = AppConfig::from_toml_str(
        "[visuals.now_playing]\nenabled=false\nfade_duration_ms=0\n\
         [visuals.now_playing.artwork]\nenabled=false\nsize=0\nradius=0\nopacity=0\n\
         [visuals.now_playing.artist]\nwidth=0\nfont_size=0\n\
         [visuals.now_playing.title]\nwidth=0\nfont_size=0\n",
    )
    .unwrap();
    let media = ShellTheme::from_config(&config).now_playing;
    assert!(!media.enabled && !media.artwork_enabled);
    assert!(media.artist_enabled && media.title_enabled);
    assert_eq!(media.fade_duration_ms, Some(0));
    assert_eq!(
        (
            media.artwork_size,
            media.artwork_radius,
            media.artwork_opacity
        ),
        (Some(0), Some(0), Some(0))
    );
    assert_eq!(
        (
            media.artist_width,
            media.title_width,
            media.artist_font_size,
            media.title_font_size
        ),
        (Some(0), Some(0), Some(0), Some(0))
    );
    assert_eq!(
        (media.artist_font_family, media.title_font_family),
        (None, None)
    );
    assert_eq!(
        (media.artist_font_weight, media.title_font_weight),
        (None, None)
    );
    assert_eq!((media.artist_color, media.title_color), (None, None));
    assert!(
        media.artwork_position.is_none()
            && media.artist_position.is_none()
            && media.title_position.is_none()
    );
}

#[test]
fn media_scaling_preserves_time_opacity_and_styles_with_sorted_backdrop_targets() {
    let config = AppConfig::from_toml_str(
        "[[visuals.backdrop]]\nname='media'\nz=3\n\
         [[visuals.backdrop]]\nname='earlier'\nz=-1\n\
         [visuals.now_playing]\nfade_duration_ms=731\n\
         [visuals.now_playing.artwork]\nrelative_to='media'\nx=-7\ny=9\nsize=83\nradius=17\nopacity=63\n\
         [visuals.now_playing.artist]\nrelative_to='earlier'\nx=3\ny=-5\nwidth=181\nfont_size=19\nfont_weight=500\n\
         [visuals.now_playing.title]\nrelative_to='media'\nx=-3\ny=7\nwidth=217\nfont_size=23\nfont_style='italic'\n",
    ).unwrap();
    let theme = ShellTheme::from_config(&config);
    let scaled = theme
        .scaled_for_render_at(RenderScale::from_units(150))
        .now_playing;
    let artwork = scaled.artwork_position.unwrap();
    let artist = scaled.artist_position.unwrap();
    let title = scaled.title_position.unwrap();
    assert_eq!(
        (artwork.target, artwork.x, artwork.y),
        (WidgetPositionTarget::Backdrop(1), -9, 11)
    );
    assert_eq!(
        (artist.target, artist.x, artist.y),
        (WidgetPositionTarget::Backdrop(0), 4, -6)
    );
    assert_eq!(
        (title.target, title.x, title.y),
        (WidgetPositionTarget::Backdrop(1), -4, 9)
    );
    assert_eq!(
        (
            scaled.artwork_size,
            scaled.artwork_radius,
            scaled.artist_width,
            scaled.title_width
        ),
        (Some(104), Some(21), Some(226), Some(271))
    );
    assert_eq!(
        (scaled.artist_font_size, scaled.title_font_size),
        (Some(24), Some(29))
    );
    assert_eq!(
        (scaled.fade_duration_ms, scaled.artwork_opacity),
        (Some(731), Some(63))
    );
    assert_eq!(
        scaled.artist_font_weight,
        theme.now_playing.artist_font_weight
    );
    assert_eq!(scaled.title_font_style, theme.now_playing.title_font_style);
    assert_eq!(theme.now_playing.artwork_size, Some(83));
}

#[test]
fn media_scaling_preserves_absent_dimensions_and_non_pixel_overrides() {
    let mut theme = ShellTheme::default();
    let media = &mut theme.now_playing;
    media.artwork_position = None;
    media.artist_position = None;
    media.title_position = None;
    media.artwork_size = None;
    media.artwork_radius = Some(0);
    media.artist_width = None;
    media.title_width = Some(0);
    media.artist_font_size = None;
    media.title_font_size = None;
    media.artist_font_family = Some("  Geom  ".into());
    media.title_color = None;
    assert_eq!(
        theme
            .scaled_for_render_at(RenderScale::from_units(180))
            .now_playing,
        theme.now_playing
    );
}

#[test]
fn media_reload_matches_fresh_pixels_geometry_and_pending_artwork_at_each_scale() {
    let initial = AppConfig::from_toml_str(
        "[visuals.now_playing.artwork]\nx=-7\nsize=83\n[visuals.now_playing.title]\nwidth=217\n",
    )
    .unwrap();
    let updated = AppConfig::from_toml_str(
        "[visuals.now_playing.artwork]\nx=-13\nsize=101\nradius=99\nopacity=63\n\
         [visuals.now_playing.artist]\ny=-91\nfont_size=27\ncolor='#AABBCC'\n\
         [visuals.now_playing.title]\ny=-53\nwidth=181\nfont_size=31\nfont_weight=700\n",
    )
    .unwrap();
    let snapshot = NowPlayingSnapshot {
        title: "A long title Ω to truncate after reload".into(),
        artist: Some("Artist Ž".into()),
        artwork_path: Some("/tmp/media-regression-cover.png".into()),
        fetched_at_unix: 1,
    };
    let mut reused = ShellState::new(ShellTheme::from_config(&initial), None, None, false);
    let mut cached =
        SoftwareBuffer::solid(FrameSize::new(640, 360), ClearColor::opaque(0, 0, 0)).unwrap();
    reused.render(&mut cached);
    let theme = ShellTheme::from_config(&updated);
    reused.apply_theme_with_username_and_weather(
        theme.clone(),
        None,
        None,
        None,
        false,
        None,
        None,
        Default::default(),
        None,
        Some(snapshot.clone()),
    );
    let mut fresh = ShellState::new_with_username_and_widgets(
        theme,
        None,
        None,
        None,
        false,
        None,
        None,
        Default::default(),
        None,
        Some(snapshot),
    );
    for shell in [&mut reused, &mut fresh] {
        shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    }
    assert_eq!(
        reused.pending_now_playing_artwork_path(),
        fresh.pending_now_playing_artwork_path()
    );
    for units in [120, 150, 180, 240] {
        let scale = RenderScale::from_units(units);
        let size = scale.frame_size(FrameSize::new(640, 360));
        let mut actual = SoftwareBuffer::solid(size, ClearColor::opaque(0, 0, 0)).unwrap();
        let mut expected = actual.clone();
        reused.render_at_scale(&mut actual, scale);
        fresh.render_at_scale(&mut expected, scale);
        assert_eq!(actual.pixels(), expected.pixels(), "units={units}");
        assert_eq!(
            reused.widget_regions_at_scale(size, scale),
            fresh.widget_regions_at_scale(size, scale)
        );
        assert_eq!(
            reused.now_playing_artwork_decode_size_at_scale(size, scale),
            fresh.now_playing_artwork_decode_size_at_scale(size, scale)
        );
    }
}

#[test]
fn grouped_media_fade_retains_default_and_runtime_duration_clamps() {
    for (configured, duration) in [(None, 450), (Some(0), 1), (Some(u64::MAX), 10_000)] {
        let mut theme = ShellTheme::default();
        theme.clock.enabled = false;
        theme.date.enabled = false;
        theme.now_playing.fade_duration_ms = configured;
        let mut shell = ShellState::new(theme, None, None, false);
        shell.set_now_playing_snapshot(Some(NowPlayingSnapshot {
            title: "Track".into(),
            artist: None,
            artwork_path: None,
            fetched_at_unix: 1,
        }));
        assert!(shell.next_animation_in(Instant::now()).is_some());
        shell.now_playing_transition.as_mut().unwrap().started_at =
            Instant::now() - Duration::from_millis(duration + 1);
        assert_eq!(
            shell.advance_animated_state_update(),
            ShellAnimationUpdate::Widget(WidgetKind::Media)
        );
        assert!(shell.now_playing_transition.is_none());
        assert!(shell.next_animation_in(Instant::now()).is_none());
    }
}
