use super::*;
use std::{fs, process::Command};
use veila_common::AppConfig;
use veila_renderer::background::{
    BackgroundAsset, load_cached_generated_render_variant, load_cached_render_variant,
};
use veila_ui::{
    ShellState, ShellTheme,
    background::{background_generated, background_treatment},
};

#[test]
fn worker_writes_match_live_backdrops_and_preserve_warm_entries() {
    if let Some(root) = std::env::var_os("VEILA_TEST_BACKDROP_JOB") {
        check(Path::new(&root));
        return;
    }
    let root = std::env::temp_dir().join(format!("veila-backdrop-job-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "background::cache::tests::worker_writes_match_live_backdrops_and_preserve_warm_entries", "--nocapture"])
        .env("VEILA_TEST_BACKDROP_JOB", &root).env("XDG_CACHE_HOME", &root).output().unwrap();
    fs::remove_dir_all(root).unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn check(root: &Path) {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<StaticBackdrops>();
    let config = AppConfig::from_toml_str(
        r##"
[background]
mode="radial"
[[visuals.backdrop]]
mode="blur"
full_height=true
width=41
inset_top=3
inset_bottom=7
rotate=-13
blur_strength=2
radius=4
border_width=1
border_color="#FFFFFF80"
[[visuals.backdrop]]
mode="solid"
show_when="now_playing"
full_width=true
full_height=true
color="#00FF00"
[[visuals.layer]]
kind="text"
text="must not enter backdrop cache"
"##,
    )
    .unwrap();
    let shell = ShellState::new(ShellTheme::from_config(&config), None, None, false);
    let treatment = background_treatment(&config.background);
    let generated = background_generated(&config.background).unwrap();
    let size = FrameSize::new(96, 64);
    let scales = [
        RenderScale::ONE,
        RenderScale::from_units(180),
        RenderScale::from_units(240),
    ];
    let wallpaper = root.join("image.png");
    SoftwareBuffer::solid(FrameSize::new(4, 2), ClearColor::rgba(0, 0, 0, 0))
        .unwrap()
        .save_png(&wallpaper)
        .unwrap();
    let fallback = ClearColor::opaque(75, 31, 139);
    let job = || {
        BackdropJob::new(
            shell.static_backdrops(),
            scales
                .iter()
                .copied()
                .map(|scale| (size, scale))
                .chain([(size, RenderScale::ONE)])
                .collect(),
        )
    };
    for file in [true, false] {
        let asset = BackgroundAsset::load(
            file.then_some(wallpaper.as_path()),
            fallback,
            (!file).then_some(generated),
            treatment,
        )
        .unwrap();
        let base = asset.render(size).unwrap();
        if file {
            job().store_file(&wallpaper, fallback, treatment, vec![(size, base.clone())]);
        } else {
            job().store_generated(generated, treatment, vec![(size, base.clone())]);
        }
        for scale in scales {
            let variant = shell.backdrop_cache_variant_at_scale(scale).unwrap();
            let loaded = if file {
                load_cached_render_variant(&wallpaper, fallback, size, treatment, &variant)
            } else {
                load_cached_generated_render_variant(generated, size, treatment, &variant)
            }
            .unwrap()
            .unwrap();
            let mut expected = base.clone();
            shell.render_static_backdrops_at_scale(&mut expected, scale);
            assert_eq!(loaded, expected, "file={file}, scale={}", scale.units());
        }
        let paths: Vec<_> = fs::read_dir(root.join("veila/backgrounds"))
            .unwrap()
            .map(|p| p.unwrap().path())
            .collect();
        let stamps: Vec<_> = paths
            .iter()
            .map(|p| fs::metadata(p).unwrap().modified().unwrap())
            .collect();
        if file {
            job().store_file(&wallpaper, fallback, treatment, vec![(size, base)]);
        } else {
            job().store_generated(generated, treatment, vec![(size, base)]);
        }
        assert_eq!(
            stamps,
            paths
                .iter()
                .map(|p| fs::metadata(p).unwrap().modified().unwrap())
                .collect::<Vec<_>>()
        );
    }
}
