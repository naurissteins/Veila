use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{ClearColor, FrameSize, SoftwareBuffer};

use super::{
    super::BackgroundScaling, BackgroundTreatment, CacheSource, GeneratedBackground, cache_path,
};
use crate::background::BackgroundGradient;

#[test]
fn round_trips_rendered_background_buffers() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-render-cache-test-{unique}"));
    fs::create_dir_all(&root).expect("cache root");

    let wallpaper = root.join("wallpaper.jpg");
    fs::write(&wallpaper, b"stub").expect("wallpaper file");

    let size = FrameSize::new(2, 1);
    let buffer = SoftwareBuffer::solid(size, ClearColor::opaque(12, 16, 24)).expect("buffer");
    store_cached_buffer_at(
        CacheSource::Path(&wallpaper),
        size,
        BackgroundTreatment::default(),
        &buffer,
        None,
        &root,
    )
    .expect("store");

    let loaded = load_cached_buffer_at(
        CacheSource::Path(&wallpaper),
        size,
        BackgroundTreatment::default(),
        None,
        &root,
    )
    .expect("load")
    .expect("cached buffer");
    assert_eq!(loaded, buffer);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn separates_variant_cache_entries() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-render-cache-variant-test-{unique}"));
    fs::create_dir_all(&root).expect("cache root");

    let wallpaper = root.join("wallpaper.jpg");
    fs::write(&wallpaper, b"stub").expect("wallpaper file");

    let size = FrameSize::new(2, 1);
    let base = SoftwareBuffer::solid(size, ClearColor::opaque(12, 16, 24)).expect("buffer");
    let layered = SoftwareBuffer::solid(size, ClearColor::opaque(40, 50, 60)).expect("buffer");
    store_cached_buffer_at(
        CacheSource::Path(&wallpaper),
        size,
        BackgroundTreatment::default(),
        &base,
        None,
        &root,
    )
    .expect("store base");
    store_cached_buffer_at(
        CacheSource::Path(&wallpaper),
        size,
        BackgroundTreatment::default(),
        &layered,
        Some("layer:v1"),
        &root,
    )
    .expect("store layered");

    let loaded_base = load_cached_buffer_at(
        CacheSource::Path(&wallpaper),
        size,
        BackgroundTreatment::default(),
        None,
        &root,
    )
    .expect("load")
    .expect("cached buffer");
    let loaded_layered = load_cached_buffer_at(
        CacheSource::Path(&wallpaper),
        size,
        BackgroundTreatment::default(),
        Some("layer:v1"),
        &root,
    )
    .expect("load")
    .expect("cached buffer");
    assert_eq!(loaded_base, base);
    assert_eq!(loaded_layered, layered);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn separates_generated_cache_entries() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-render-generated-cache-test-{unique}"));
    fs::create_dir_all(&root).expect("cache root");

    let generated = GeneratedBackground::Gradient(BackgroundGradient {
        top_left: ClearColor::opaque(255, 0, 0),
        top_right: ClearColor::opaque(0, 255, 0),
        bottom_left: ClearColor::opaque(0, 0, 255),
        bottom_right: ClearColor::opaque(255, 255, 255),
    });
    let size = FrameSize::new(2, 1);
    let base = SoftwareBuffer::solid(size, ClearColor::opaque(12, 16, 24)).expect("buffer");
    let layered = SoftwareBuffer::solid(size, ClearColor::opaque(40, 50, 60)).expect("buffer");
    store_cached_buffer_at(
        CacheSource::Generated(generated),
        size,
        BackgroundTreatment::default(),
        &base,
        None,
        &root,
    )
    .expect("store base");
    store_cached_buffer_at(
        CacheSource::Generated(generated),
        size,
        BackgroundTreatment::default(),
        &layered,
        Some("layer:v1"),
        &root,
    )
    .expect("store layered");

    let loaded_base = load_cached_buffer_at(
        CacheSource::Generated(generated),
        size,
        BackgroundTreatment::default(),
        None,
        &root,
    )
    .expect("load")
    .expect("cached buffer");
    let loaded_layered = load_cached_buffer_at(
        CacheSource::Generated(generated),
        size,
        BackgroundTreatment::default(),
        Some("layer:v1"),
        &root,
    )
    .expect("load")
    .expect("cached buffer");
    assert_eq!(loaded_base, base);
    assert_eq!(loaded_layered, layered);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn separates_cache_entries_by_background_scaling() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("veila-render-scale-cache-test-{unique}"));
    fs::create_dir_all(&root).expect("cache root");

    let wallpaper = root.join("wallpaper.jpg");
    fs::write(&wallpaper, b"stub").expect("wallpaper file");
    let size = FrameSize::new(1920, 1080);

    let fill = cache_path(
        CacheSource::Path(&wallpaper),
        size,
        BackgroundTreatment {
            scaling: BackgroundScaling::Fill,
            ..BackgroundTreatment::default()
        },
        None,
        Some(&root),
    )
    .expect("fill key");
    let fit = cache_path(
        CacheSource::Path(&wallpaper),
        size,
        BackgroundTreatment {
            scaling: BackgroundScaling::Fit,
            ..BackgroundTreatment::default()
        },
        None,
        Some(&root),
    )
    .expect("fit key");

    assert_ne!(fill, fit);

    let _ = fs::remove_dir_all(root);
}

fn load_cached_buffer_at(
    source: CacheSource<'_>,
    size: FrameSize,
    treatment: BackgroundTreatment,
    variant: Option<&str>,
    cache_home: &Path,
) -> crate::Result<Option<SoftwareBuffer>> {
    let cache_path = cache_path(source, size, treatment, variant, Some(cache_home))?;
    Ok(super::read_buffer(&cache_path, size))
}

fn store_cached_buffer_at(
    source: CacheSource<'_>,
    size: FrameSize,
    treatment: BackgroundTreatment,
    buffer: &SoftwareBuffer,
    variant: Option<&str>,
    cache_home: &Path,
) -> crate::Result<()> {
    let cache_path = cache_path(source, size, treatment, variant, Some(cache_home))?;
    super::write_buffer(&cache_path, size, buffer)
}
