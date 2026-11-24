use veila_common::AppConfig;
use veila_renderer::{ClearColor, FrameSize, RenderScale, SoftwareBuffer};

use crate::shell::{ShellState, ShellTheme};

const DUSK_LAYERS: &str = r##"
    [[visuals.backdrop]]
    name = "auth_glass"
    width = 545
    height = 690
    halign = "center"
    valign = "center"

    [[visuals.layer]]
    name = "session_tag"
    kind = "text"
    text = "S E S S I O N   L O C K E D"
    font_family = "Google Sans Flex"
    font_size = 15
    color = "#00FF00"
    halign = "left"
    valign = "top"
    x = 92
    y = 61
    z = 1

    [[visuals.layer]]
    name = "ghost_wordmark"
    kind = "text"
    text = "VEILA"
    font_family = "Geom"
    font_weight = 600
    font_size = 420
    color = "rgba(255, 0, 0, 0.035)"
    halign = "left"
    valign = "bottom"
    x = -30
    y = 110
    z = 0

    [[visuals.layer]]
    name = "auth_hint"
    kind = "text"
    text = "Enter password to unlock"
    font_family = "Google Sans Flex"
    font_size = 17
    color = "#0000FF"
    halign = "center"
    valign = "top"
    y = 292
    relative_to = "auth_glass"
    z = 2
"##;

#[test]
fn startup_overlay_draws_the_complete_dusk_scene() {
    let config = AppConfig::from_toml_str(DUSK_LAYERS).unwrap();
    let shell = ShellState::new(ShellTheme::from_config(&config), None, None, true);

    for scale in [RenderScale::ONE, RenderScale::from_units(180)] {
        let mut startup = frame(scale);
        assert!(shell.render_startup_static_overlay_at_scale(&mut startup, scale));
        assert!(has_color(&startup, 1), "session tag");
        assert!(has_color(&startup, 0), "auth hint");
        assert!(has_color(&startup, 2), "wordmark");

        let mut full = frame(scale);
        shell.render_static_overlay_at_scale(&mut full, scale);
        assert_eq!(startup.pixels(), full.pixels());
    }
}

#[test]
fn startup_overlay_defers_additional_wordmarks() {
    let mut config = AppConfig::from_toml_str(DUSK_LAYERS).unwrap();
    let mut second_wordmark = config
        .visuals
        .layer
        .iter()
        .find(|layer| layer.name.as_deref() == Some("ghost_wordmark"))
        .unwrap()
        .clone();
    second_wordmark.text = Some("LOCK".into());
    second_wordmark.position.halign = Some(veila_common::HorizontalAlign::Right);
    config.visuals.layer.push(second_wordmark);
    let shell = ShellState::new(ShellTheme::from_config(&config), None, None, true);
    let scale = RenderScale::from_units(180);
    let mut startup = frame(scale);
    let mut full = frame(scale);

    assert!(!shell.render_startup_static_overlay_at_scale(&mut startup, scale));
    assert!(has_color(&startup, 2), "first wordmark");
    shell.render_static_overlay_at_scale(&mut full, scale);
    assert_ne!(startup.pixels(), full.pixels());
}

#[test]
fn startup_wordmark_respects_the_scaled_raster_budget() {
    let mut config = AppConfig::from_toml_str(DUSK_LAYERS).unwrap();
    let wordmark = config
        .visuals
        .layer
        .iter_mut()
        .find(|layer| layer.name.as_deref() == Some("ghost_wordmark"))
        .unwrap();
    wordmark.text = Some("LOCKED".into());
    let shell = ShellState::new(ShellTheme::from_config(&config), None, None, true);

    assert!(
        shell
            .render_startup_static_overlay_at_scale(&mut frame(RenderScale::ONE), RenderScale::ONE)
    );
    let scale = RenderScale::from_units(180);
    let mut startup = frame(scale);
    assert!(!shell.render_startup_static_overlay_at_scale(&mut startup, scale));
    assert!(!has_color(&startup, 2), "over-budget wordmark deferred");
}

#[test]
fn startup_text_layers_share_a_budget_in_device_pixels() {
    let mut config = AppConfig::from_toml_str(DUSK_LAYERS).unwrap();
    config
        .visuals
        .layer
        .retain(|layer| layer.name.as_deref() != Some("ghost_wordmark"));
    for layer in &mut config.visuals.layer {
        layer.font_size = Some(32);
    }
    let shell = ShellState::new(ShellTheme::from_config(&config), None, None, true);

    assert!(
        shell
            .render_startup_static_overlay_at_scale(&mut frame(RenderScale::ONE), RenderScale::ONE)
    );
    let scale = RenderScale::from_units(180);
    let mut startup = frame(scale);
    assert!(!shell.render_startup_static_overlay_at_scale(&mut startup, scale));
    assert!(has_color(&startup, 1), "first label fits");
    assert!(
        !has_color(&startup, 0),
        "second label exceeds the remainder"
    );
}

fn frame(scale: RenderScale) -> SoftwareBuffer {
    SoftwareBuffer::solid(
        scale.frame_size(FrameSize::new(1280, 800)),
        ClearColor::opaque(0, 0, 0),
    )
    .unwrap()
}

fn has_color(buffer: &SoftwareBuffer, channel: usize) -> bool {
    buffer.pixels().as_chunks::<4>().0.iter().any(|pixel| {
        pixel[channel] > 0 && pixel[(channel + 1) % 3] == 0 && pixel[(channel + 2) % 3] == 0
    })
}
