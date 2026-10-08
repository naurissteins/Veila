use crate::shell::{ShellKey, ShellState, ShellTheme};
use veila_common::{AppConfig, NowPlayingSnapshot, WeatherCondition, WeatherSnapshot, WeatherUnit};
use veila_renderer::{ClearColor, FrameSize, RenderScale, SoftwareBuffer, copy_rect_from};

use super::WidgetDamage;

fn shell(config: &str) -> ShellState {
    let config = AppConfig::from_toml_str(&format!("theme='default'\n{config}")).unwrap();
    let mut shell = ShellState::new_with_username_and_widgets(
        ShellTheme::from_config(&config),
        None,
        Some("Āna Ω".into()),
        None,
        true,
        Some("Riga".into()),
        Some(WeatherSnapshot {
            temperature_celsius: 7,
            condition: WeatherCondition::Rain,
            fetched_at_unix: 0,
        }),
        WeatherUnit::Celsius,
        None,
        Some(NowPlayingSnapshot {
            title: "Track title".into(),
            artist: None,
            artwork_path: None,
            fetched_at_unix: 0,
        }),
    );
    shell.set_preview_time(time::OffsetDateTime::UNIX_EPOCH);
    shell.set_keyboard_layout_label(Some("EN".into()));
    shell.handle_key(ShellKey::Character('x'));
    shell
}

fn verify_change(
    shell: &mut ShellState,
    units: u32,
    expected_full: bool,
    change: impl FnOnce(&mut ShellState),
) {
    let scale = RenderScale::from_units(units);
    let size = scale.frame_size(FrameSize::new(960, 640));
    let mut base = SoftwareBuffer::solid(size, ClearColor::opaque(9, 12, 20)).unwrap();
    shell.render_static_overlay_at_scale(&mut base, scale);
    let previous = shell.widget_regions_at_scale(size, scale);
    let mut actual = base.clone();
    shell.render_dynamic_overlay_at_scale(&mut actual, scale);
    let revision = shell.static_scene_revision();
    change(shell);
    assert_eq!(
        revision,
        shell.static_scene_revision(),
        "base must stay valid"
    );
    let current = shell.widget_regions_at_scale(size, scale);
    let damage = current.auth_damage_since(previous);
    assert_eq!(damage == WidgetDamage::Full, expected_full, "{current:?}");
    let mut expected = base.clone();
    shell.render_dynamic_overlay_at_scale(&mut expected, scale);
    match damage {
        WidgetDamage::Full => {
            actual = base;
            shell.render_dynamic_overlay_at_scale(&mut actual, scale);
        }
        WidgetDamage::Region(rect) => {
            assert!(rect.x >= 0 && rect.y >= 0);
            assert!(rect.right() <= size.width as i32 && rect.bottom() <= size.height as i32);
            copy_rect_from(&base, &mut actual, rect).unwrap();
            shell.render_auth_dirty_overlay_at_scale(&mut actual, scale);
        }
        WidgetDamage::Skip => panic!("auth updates must be repainted"),
    }
    let mismatch = actual
        .pixels()
        .iter()
        .zip(expected.pixels())
        .position(|(a, b)| a != b);
    assert_eq!(mismatch, None, "first differing byte at scale {units}");
}

#[test]
fn overlapping_clock_typing_uses_full_frame_and_preserves_clock_pixels() {
    let config = "[visuals.input]\nhalign='left'\nvalign='top'\nx=24\ny=32\nwidth=650\n";
    for units in [120, 150, 180, 240] {
        let mut shell = shell(config);
        verify_change(&mut shell, units, units != 240, |shell| {
            shell.handle_key(ShellKey::Character('y'));
        });
    }
}

#[test]
fn auth_overlaps_with_date_media_keyboard_and_weather_use_full_frames() {
    for widget in [
        "clock",
        "date",
        "now_playing.title",
        "keyboard",
        "weather.temperature",
    ] {
        let config = format!(
            "[weather]\nenabled={}\n[visuals.now_playing]\nenabled={}\n\
             [visuals.input]\nhalign='left'\nvalign='bottom'\nx=24\ny=-32\n\
             [visuals.{widget}]\nenabled=true\nhalign='left'\nvalign='bottom'\nx=24\ny=-32\n",
            widget == "weather.temperature",
            widget == "now_playing.title",
        );
        for units in [120, 150, 180, 240] {
            let mut shell = shell(&config);
            let scale = RenderScale::from_units(units);
            let regions =
                shell.widget_regions_at_scale(scale.frame_size(FrameSize::new(960, 640)), scale);
            let neighbor = match widget {
                "clock" | "date" => regions.header,
                "now_playing.title" => regions.media,
                "keyboard" => regions.indicators,
                _ => regions.weather,
            };
            assert!(super::super::overlaps(
                regions.auth.unwrap(),
                neighbor.unwrap()
            ));
            verify_change(&mut shell, units, true, |shell| {
                shell.handle_key(ShellKey::Character('y'));
            });
        }
    }
}

#[test]
fn isolated_input_typing_and_deletion_keep_partial_full_pixel_parity() {
    for mode in ["inline", "external", "hidden"] {
        for units in [120, 150, 180, 240] {
            let mut shell = shell(&format!(
                "[visuals.input]\nhalign='left'\nvalign='bottom'\nx=24\ny=-32\n\
                 [visuals.status]\nmode='{mode}'\n"
            ));
            verify_change(&mut shell, units, false, |shell| {
                shell.handle_key(ShellKey::Character('y'));
            });
            verify_change(&mut shell, units, false, |shell| {
                shell.handle_key(ShellKey::Backspace);
            });
        }
    }
}

#[test]
fn shrinking_external_rejection_status_clears_previous_pixels() {
    for units in [120, 150, 180, 240] {
        let mut shell = shell(
            "[visuals.input]\nhalign='left'\nvalign='bottom'\nx=24\ny=-32\n\
             [visuals.status]\nmode='external'\n",
        );
        shell.authentication_rejected_with_message(None, None, Some("A long rejection Ω".into()));
        verify_change(&mut shell, units, false, |shell| {
            shell.authentication_rejected_with_message(None, None, Some("No".into()));
        });
    }
}

#[test]
fn preview_grid_uses_full_frame_for_auth_updates() {
    let mut shell = shell("[visuals.input]\nhalign='left'\nvalign='bottom'\nx=24\ny=-32\n");
    shell.set_preview_grid_enabled(true);
    verify_change(&mut shell, 150, true, |shell| {
        shell.handle_key(ShellKey::Character('y'));
    });
}

#[test]
fn unguarded_auth_repaint_reproduces_clock_erasure() {
    let mut shell = shell("[visuals.input]\nhalign='left'\nvalign='top'\nx=24\ny=32\nwidth=650\n");
    let size = FrameSize::new(960, 640);
    let mut base = SoftwareBuffer::solid(size, ClearColor::opaque(9, 12, 20)).unwrap();
    shell.render_static_overlay_at_scale(&mut base, RenderScale::ONE);
    let previous = shell.widget_regions_at_scale(size, RenderScale::ONE);
    let mut legacy = base.clone();
    shell.render_dynamic_overlay_at_scale(&mut legacy, RenderScale::ONE);
    shell.handle_key(ShellKey::Character('y'));
    let current = shell.widget_regions_at_scale(size, RenderScale::ONE);
    copy_rect_from(&base, &mut legacy, current.auth.unwrap()).unwrap();
    shell.render_auth_dirty_overlay_at_scale(&mut legacy, RenderScale::ONE);
    let mut full = base;
    shell.render_dynamic_overlay_at_scale(&mut full, RenderScale::ONE);
    assert!(legacy.pixels() != full.pixels());
    assert_eq!(current.auth_damage_since(previous), WidgetDamage::Full);
}

#[test]
fn dynamic_backdrops_preserve_full_paint_order_on_auth_updates() {
    for rotate in [0, 45] {
        let mut shell = shell(&format!(
            "[visuals.now_playing]\nenabled=true\n\
             [visuals.input]\nhalign='left'\nvalign='bottom'\nx=24\ny=-32\n\
             [[visuals.backdrop]]\nenabled=true\nshow_when='now_playing'\nmode='blur'\n\
             color='#20304080'\nwidth=400\nheight=100\nhalign='left'\nvalign='bottom'\n\
             x=24\ny=-32\nrotate={rotate}\n"
        ));
        verify_change(&mut shell, 150, true, |shell| {
            shell.handle_key(ShellKey::Character('y'));
        });
    }
}
