use std::{fs, path::Path, process::Command};

use veila_common::AppConfig;
use veila_renderer::{
    ClearColor, FrameSize, SoftwareBuffer,
    background::{
        BackgroundAsset, GeneratedBackground, load_cached_generated_render_variant,
        load_cached_render_variant,
    },
};
use veila_ui::{ShellState, ShellTheme, background::background_treatment};

use super::{
    PrewarmSize, ScenePrewarmConfig, prewarm_generated_backgrounds, prewarm_layered_backgrounds,
};

#[test]
fn prewarmed_backdrops_match_curtain_keys_and_pixels() {
    if std::env::var_os("VEILA_TEST_PREWARM_CACHE").is_some() {
        check_theme_caches();
        return;
    }
    let root = std::env::temp_dir().join(format!("veila-prewarm-parity-{}", std::process::id()));
    fs::create_dir_all(&root).expect("isolated cache");
    let output = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "app::prewarm::parity_tests::prewarmed_backdrops_match_curtain_keys_and_pixels",
            "--nocapture",
        ])
        .env("VEILA_TEST_PREWARM_CACHE", &root)
        .env("XDG_CACHE_HOME", &root)
        .env("HOME", &root)
        .output()
        .expect("isolated cache test process");
    fs::remove_dir_all(root).expect("remove test cache");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn check_theme_caches() {
    let root = std::env::var_os("VEILA_TEST_PREWARM_CACHE").expect("test root");
    let wallpaper = Path::new(&root).join("wallpaper.png");
    SoftwareBuffer::solid(FrameSize::new(8, 8), ClearColor::opaque(37, 59, 83))
        .expect("wallpaper")
        .save_png(&wallpaper)
        .expect("save wallpaper");
    let themes = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/themes");
    let mut count = 0;
    for entry in fs::read_dir(themes).expect("bundled themes") {
        let path = entry.expect("theme entry").path();
        if path.extension().is_none_or(|extension| extension != "toml") {
            continue;
        }
        let mut config = AppConfig::from_toml_str(&fs::read_to_string(&path).expect("theme"))
            .expect("valid theme");
        config.visuals.layer.clear();
        check_config_caches(&config, &wallpaper);
        count += 1;
    }
    assert!(count > 0, "exercise every bundled theme");
    for raw in ["", COMPLEX_BACKDROPS, STATIC_LAYER] {
        let mut config = AppConfig::from_toml_str(raw).expect("fixture");
        if raw.is_empty() {
            config.visuals.backdrop.clear();
        }
        check_config_caches(&config, &wallpaper);
    }
}

fn check_config_caches(config: &AppConfig, wallpaper: &Path) {
    let prewarm = ScenePrewarmConfig::from_config(config).into_shell();
    let curtain = ShellState::new_with_username_and_widgets(
        ShellTheme::from_config(config),
        Some(config.visuals.input_placeholder()),
        config.visuals.username_text().map(str::to_owned),
        config.avatar_image_path().map(Path::to_path_buf),
        config.visuals.username_enabled(),
        config.weather.normalized_location(),
        None,
        config.weather.unit,
        None,
        None,
    );
    let generated = veila_ui::background::background_generated(
        &AppConfig::from_toml_str("[background]\nmode = 'radial'")
            .expect("generated config")
            .background,
    )
    .expect("generated background");
    let sizes = [1, 2].map(|scale| PrewarmSize {
        buffer: FrameSize::new(320 * scale as u32, 180 * scale as u32),
        scale,
    });
    let treatment = background_treatment(&config.background);
    let fallback = ClearColor::opaque(0, 0, 0);
    prewarm_layered_backgrounds(wallpaper, fallback, treatment, &prewarm, &sizes);
    prewarm_generated_backgrounds(generated, treatment, &prewarm, &sizes)
        .expect("generated prewarm");
    for size in sizes {
        assert_eq!(
            prewarm.backdrop_cache_variant_scaled(size.scale as u32),
            curtain.backdrop_cache_variant_scaled(size.scale as u32)
        );
        check_cached_pixels(&curtain, config, wallpaper, generated, size);
    }
    if let Some(report) =
        prewarm_layered_backgrounds(wallpaper, fallback, treatment, &prewarm, &sizes)
    {
        assert_eq!(report.warmed_sizes, 0, "reuse the existing file cache");
        assert!(report.cache_hits > 0);
    }
    let report = prewarm_generated_backgrounds(generated, treatment, &prewarm, &sizes)
        .expect("repeat generated prewarm");
    assert_eq!(report.rendered.summary.warmed_sizes, 0);
    if let Some(report) = report.layered {
        assert_eq!(report.warmed_sizes, 0, "reuse the existing generated cache");
        assert!(report.cache_hits > 0);
    }
}

fn check_cached_pixels(
    curtain: &ShellState,
    config: &AppConfig,
    wallpaper: &Path,
    generated: GeneratedBackground,
    size: PrewarmSize,
) {
    let scale = size.scale as u32;
    let treatment = background_treatment(&config.background);
    let variants = [
        (curtain.backdrop_cache_variant_scaled(scale), false),
        (curtain.static_scene_cache_variant(scale), true),
    ];
    for (variant, static_scene) in variants {
        let Some(variant) = variant else { continue };
        for file_mode in [true, false] {
            let cached = if file_mode {
                load_cached_render_variant(wallpaper, size.buffer, treatment, &variant)
            } else {
                load_cached_generated_render_variant(generated, size.buffer, treatment, &variant)
            }
            .expect("read warmed cache")
            .expect("curtain key must hit prewarm cache");
            let asset = BackgroundAsset::load(
                file_mode.then_some(wallpaper),
                ClearColor::opaque(0, 0, 0),
                (!file_mode).then_some(generated),
                treatment,
            )
            .expect("background");
            let mut expected = asset.render(size.buffer).expect("render background");
            curtain.render_static_backdrops_scaled(&mut expected, scale);
            if static_scene {
                curtain.render_static_overlay_scaled(&mut expected, scale);
            }
            assert!(
                cached.pixels() == expected.pixels(),
                "cache pixels differ: file={file_mode}, scale={scale}, variant={variant}"
            );
        }
    }
}

const COMPLEX_BACKDROPS: &str = r##"
[[visuals.backdrop]]
mode = "solid"
color = "#31577B80"
full_width = true
full_height = true
inset_left = 7
inset_right = 13
inset_top = 11
inset_bottom = 17
rotate = -19
border_width = 3
border_color = "#FFFFFF80"
radius = 9
z = 2
[[visuals.backdrop]]
mode = "blur"
width = 113
height = 77
halign = "right"
valign = "bottom"
x = -5
y = -3
blur_strength = 4
z = -1
[[visuals.backdrop]]
enabled = false
color = "#FF0000"
[[visuals.backdrop]]
show_when = "battery"
color = "#FF0000"
[[visuals.backdrop]]
show_when = "weather"
color = "#00FF00"
[[visuals.backdrop]]
show_when = "now_playing"
color = "#0000FF"
"##;

const STATIC_LAYER: &str = r##"
[visuals.avatar]
enabled = false
[[visuals.backdrop]]
mode = "solid"
color = "#23456780"
full_width = true
full_height = true
[[visuals.layer]]
kind = "text"
text = "Veila"
font_size = 16
"##;
