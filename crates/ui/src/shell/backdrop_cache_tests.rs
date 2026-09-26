use veila_common::AppConfig;

use super::{ShellState, ShellTheme};

#[test]
fn scaled_backdrop_variant_preserves_existing_disk_keys() {
    let config = AppConfig::from_toml_str(
        r#"
        [[visuals.backdrop]]
        mode = "blur"
        width = 540
        height = 600
        halign = "left"
        valign = "center"
        full_height = true
        inset_top = 24
        inset_bottom = 36
        rotate = 12
        color = "rgba(0, 0, 0, 0.1)"
        blur_strength = 12
        border_width = 1
        border_color = "rgba(255, 255, 255, 0.094)"
        "#,
    )
    .expect("config");
    let mut shell = ShellState::new(ShellTheme::from_config(&config), None, None, false);
    let base =
        "backdrop:v2:Blur:0:1:Left:1:0:0:0:1:24:36:0:0:540:600:12:0:0:0:0:26:12:0:1:255:255:255:24";
    assert_eq!(
        shell.backdrop_cache_variant_scaled(0).as_deref(),
        Some(base)
    );
    assert_eq!(
        shell.backdrop_cache_variant_scaled(1).as_deref(),
        Some(base)
    );
    assert_eq!(
        shell.backdrop_cache_variant_scaled(2),
        Some(format!("{base}:render-scale:2"))
    );
    shell.activate_emergency();
    assert_eq!(shell.backdrop_cache_variant_scaled(2), None);
}

#[test]
fn absent_backdrops_have_no_scaled_cache_variant() {
    let mut config = AppConfig::from_toml_str("").expect("config");
    config.visuals.backdrop.clear();
    let shell = ShellState::new(ShellTheme::from_config(&config), None, None, false);
    assert_eq!(shell.backdrop_cache_variant_scaled(2), None);
}
