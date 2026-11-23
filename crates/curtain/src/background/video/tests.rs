#[cfg(feature = "video-prototype")]
use super::reader::video_header;
use super::*;
use veila_common::AppConfig;
use veila_renderer::ClearColor;
use veila_ui::{ShellState, ShellTheme};

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
