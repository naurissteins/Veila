#[cfg(feature = "video-prototype")]
use super::reader::video_header;
use super::*;
use veila_common::AppConfig;
use veila_renderer::ClearColor;
use veila_ui::{ShellState, ShellTheme, background::background_treatment};

#[test]
fn prepared_video_backdrop_matches_shell_at_fractional_scale_and_can_be_rebuilt() {
    let config = AppConfig::from_toml_str(
        r##"
[[visuals.backdrop]]
mode="blur"
width=30
full_height=true
blur_strength=3
color="#20006080"
[[visuals.backdrop]]
mode="solid"
show_when="now_playing"
full_width=true
full_height=true
color="#00FF00"
"##,
    )
    .unwrap();
    let shell = ShellState::new(ShellTheme::from_config(&config), None, None, false);
    let scale = RenderScale::from_units(180);
    let size = FrameSize::new(96, 66);
    let pixels = (0..size.width * size.height)
        .flat_map(|index| [40, (index % 17 * 11) as u8, 120, 255])
        .collect();
    let background = SoftwareBuffer::from_argb8888_pixels(size, pixels).unwrap();
    let mut expected = background.clone();
    shell.render_static_backdrops_at_scale(&mut expected, scale);
    assert_ne!(expected, background);

    let surface = SurfaceSize::new_with_fractional_scale(64, 44, 2, Some(180));
    let mut frame = VideoFrame::prepare(
        background,
        VideoGeometry::for_surface(surface),
        BackgroundTreatment::default(),
        &shell.static_backdrops(),
    );
    assert!(frame.fits(surface));
    assert!(!frame.fits(SurfaceSize::new_with_fractional_scale(96, 64, 1, None)));
    let mut first = frame.take_scene_buffer();
    assert_eq!(first, expected);
    first.clear(ClearColor::opaque(0, 0, 0));
    assert_eq!(frame.take_scene_buffer(), expected);
}

#[test]
fn video_dimming_and_tint_apply_before_backdrops_on_both_render_paths() {
    let config = AppConfig::from_toml_str(
        r##"
[background]
dim_strength = 54
tint = "#20408080"
[[visuals.backdrop]]
mode = "solid"
color = "#FF0000"
full_width = true
height = 4
valign = "bottom"
"##,
    )
    .unwrap();
    let shell = ShellState::new(ShellTheme::from_config(&config), None, None, false);

    for surface in [
        SurfaceSize::new_with_fractional_scale(128, 64, 1, None),
        SurfaceSize::new_with_fractional_scale(1921, 64, 1, None),
    ] {
        let geometry = VideoGeometry::for_surface(surface);
        let background =
            SoftwareBuffer::solid(geometry.buffer, ClearColor::opaque(255, 255, 255)).unwrap();
        let frame = VideoFrame::prepare(
            background,
            geometry,
            background_treatment(&config.background),
            &shell.static_backdrops(),
        );
        let pixels = frame.background.pixels();
        assert_eq!(&pixels[..4], &[122, 90, 74, 255]);
        assert_eq!(&pixels[pixels.len() - 4..], &[0, 0, 255, 255]);
        if let Some(scene) = frame.scene_buffer {
            assert_eq!(scene, frame.background);
        }
    }
}

#[test]
fn password_input_extends_the_video_hold_until_typing_stops() {
    let mut video = Video {
        reader: Some(VideoReader::test_handle()),
        ..Video::default()
    };
    let first_key = Instant::now();
    video.prioritize_input(first_key);
    let next_key = first_key + INPUT_HOLD / 2;
    video.prioritize_input(next_key);

    assert!(video.hold_frame_for_input(first_key + INPUT_HOLD));
    assert!(!video.hold_frame_for_input(next_key + INPUT_HOLD));
    assert!(video.input_hold_until.is_none());
}

#[cfg(feature = "video-prototype")]
#[test]
fn video_header_encodes_fixed_bgra_dimensions() {
    assert_eq!(
        video_header(FrameSize::new(1366, 768)),
        *b"VLP1\x56\x05\0\0\0\x03\0\0"
    );
}
