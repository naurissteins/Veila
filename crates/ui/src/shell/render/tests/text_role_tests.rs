use super::*;
use crate::{ClockTheme, DateTheme, InputTheme, RevealTheme, StatusTheme, UsernameTheme};
use veila_common::FontStyle;
use veila_renderer::text::FontStyle as RendererFontStyle;

#[test]
fn text_roles_keep_independent_size_limits() {
    for size in [0, 1, 511, 512, 513, 1024, 1025, u32::MAX] {
        let theme = ShellTheme {
            now_playing: crate::NowPlayingTheme {
                title_font_size: Some(size),
                artist_font_size: Some(size),
                ..ShellTheme::default().now_playing
            },
            clock: ClockTheme {
                font_size: Some(size),
                meridiem_font_size: Some(size),
                ..ShellTheme::default().clock
            },
            date: DateTheme {
                font_size: Some(size),
                ..ShellTheme::default().date
            },
            username: UsernameTheme {
                font_size: Some(size),
                ..ShellTheme::default().username
            },
            input: InputTheme {
                font_size: Some(size),
                ..ShellTheme::default().input
            },
            reveal: RevealTheme {
                font_size: Some(size),
                ..ShellTheme::default().reveal
            },
            keyboard_size: Some(size),
            weather_temperature_font_size: Some(size),
            weather_location_font_size: Some(size),
            ..ShellTheme::default()
        };
        let shell = ShellState::new(theme, None, None, true);
        let context = shell.render_context();
        let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);
        assert_eq!(
            context.clock_text_style(metrics).font_size_px,
            Some(size.clamp(1, 1024))
        );
        let roles = [
            context.clock_meridiem_text_style(metrics),
            context.date_text_style(),
            context.username_text_style(),
            context.placeholder_text_style(),
            context.revealed_secret_text_style(),
            context.reveal_text_style(),
            context.input_status_text_style(),
            context.keyboard_layout_text_style(),
            context.weather_temperature_text_style(),
            context.weather_location_text_style(),
            context.now_playing_title_text_style(),
            context.now_playing_artist_text_style(),
        ];
        for style in roles {
            assert_eq!(style.font_size_px, Some(size.clamp(1, 512)));
        }
    }
}

#[test]
fn reveal_font_inherits_only_missing_properties_from_input() {
    let theme = ShellTheme {
        input: InputTheme {
            font_family: Some(String::from("sans-serif")),
            font_weight: Some(700),
            font_style: Some(FontStyle::Italic),
            font_size: Some(24),
            ..ShellTheme::default().input
        },
        reveal: RevealTheme {
            font_weight: Some(400),
            font_style: Some(FontStyle::Normal),
            ..ShellTheme::default().reveal
        },
        ..ShellTheme::default()
    };
    let shell = ShellState::new(theme, None, None, true);
    let context = shell.render_context();
    let input = context.placeholder_text_style();
    let reveal = context.reveal_text_style();
    assert_eq!(reveal.font_family, input.font_family);
    assert_eq!(reveal.font_size_px, Some(24));
    assert_eq!(reveal.font_weight, Some(400));
    assert_eq!(reveal.font_style, Some(RendererFontStyle::Normal));
    assert_eq!(input.font_weight, Some(700));
    assert_eq!(input.font_style, Some(RendererFontStyle::Italic));
}

#[test]
fn input_and_floating_status_keep_color_policy_for_every_auth_state() {
    let now = std::time::Instant::now();
    let states = [
        ShellStatus::Idle,
        ShellStatus::Challenge {
            text: String::from("Challenge"),
            echo: false,
        },
        ShellStatus::Notice {
            text: String::from("Notice"),
        },
        ShellStatus::Pending {
            started_at: now,
            visible_after: now,
            shown: true,
            displayed_phase: 0,
        },
        ShellStatus::Rejected {
            retry_until: None,
            displayed_retry_seconds: None,
            failed_attempts: None,
            message: None,
        },
    ];
    let neutral = ClearColor::rgba(1, 2, 3, 90);
    let pending = ClearColor::rgba(4, 5, 6, 91);
    let rejected = ClearColor::rgba(7, 8, 9, 92);
    let expected_colors = [neutral, neutral, neutral, pending, rejected];
    for (status, expected) in states.into_iter().zip(expected_colors) {
        let theme = ShellTheme {
            input: InputTheme {
                font_size: Some(31),
                font_weight: Some(500),
                ..ShellTheme::default().input
            },
            status: StatusTheme {
                color: Some(neutral),
                pending_color: Some(pending),
                rejected_color: Some(rejected),
                ..ShellTheme::default().status
            },
            ..ShellTheme::default()
        };
        let mut shell = ShellState::new(theme, None, None, true);
        shell.status = status;
        let context = shell.render_context();
        let floating = context.status_text_style();
        let input = context.input_status_text_style();
        assert_eq!(floating.color, expected);
        assert_eq!(input.color, expected);
        assert_eq!(floating.font_size_px, None);
        assert_eq!(floating.scale, 2);
        assert_eq!(input.font_size_px, Some(31));
        assert_eq!(input.font_weight, Some(500));
        assert_eq!(input.line_spacing, 0);
    }
}
