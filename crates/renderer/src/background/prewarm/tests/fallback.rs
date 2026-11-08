use super::{COLOR, FileBackgroundPrewarm, SIZE, isolated, source_cache};
use crate::{ClearColor, FrameSize, SoftwareBuffer, background::BackgroundScaling};

#[test]
fn colour_only_change_rewarms_transparent_wallpaper() {
    check_fallback_change("colour_only_change_rewarms_transparent_wallpaper", true);
}

#[test]
fn colour_only_change_rewarms_fit_bars() {
    check_fallback_change("colour_only_change_rewarms_fit_bars", false);
}

fn check_fallback_change(test: &str, transparent: bool) {
    let Some(root) = isolated(&format!("fallback::{test}")) else {
        return;
    };
    let path = root.join("wallpaper.png");
    let pixel = if transparent {
        [0, 0, 0, 0]
    } else {
        [37, 59, 83, 255]
    };
    image::RgbaImage::from_pixel(1, 1, image::Rgba(pixel))
        .save(&path)
        .unwrap();
    let treatment = crate::background::BackgroundTreatment {
        scaling: BackgroundScaling::Fit,
        ..Default::default()
    };
    let mut black = FileBackgroundPrewarm::new(&path, ClearColor::opaque(0, 0, 0), treatment);
    assert_eq!(black.prewarm_rendered(&[SIZE]).unwrap().warmed_sizes, 1);
    let decoded = std::fs::read(source_cache(&root)).unwrap();
    let mut coloured = FileBackgroundPrewarm::new(&path, COLOR, treatment);
    let changed = coloured.prewarm_rendered(&[SIZE]).unwrap();
    assert_eq!(
        changed.warmed_sizes, 1,
        "fallback-only changes must miss rendered caches"
    );
    assert_eq!(changed.cache_hits, 0);
    assert_eq!(std::fs::read(source_cache(&root)).unwrap(), decoded);
    let expected = coloured.render(SIZE).unwrap();
    assert_eq!(
        crate::background::load_cached_render(&path, COLOR, SIZE, treatment).unwrap(),
        Some(expected.clone())
    );
    if transparent {
        assert_eq!(expected, SoftwareBuffer::solid(SIZE, COLOR).unwrap());
    } else {
        assert_eq!(&expected.pixels()[..4], &[83, 59, 37, 255]);
        assert_eq!(expected.size(), FrameSize::new(16, 8));
    }
    assert_eq!(coloured.prewarm_rendered(&[SIZE]).unwrap().cache_hits, 1);
}
