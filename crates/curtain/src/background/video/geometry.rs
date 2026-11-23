use veila_renderer::{FrameSize, RenderScale};

use crate::state::SurfaceSize;

const MAX_WIDTH: u64 = 1920;
const MAX_HEIGHT: u64 = 1080;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VideoGeometry {
    pub(super) buffer: FrameSize,
    pub(super) scale: RenderScale,
    pub(super) scaled: bool,
}

impl VideoGeometry {
    pub(super) fn for_surface(size: SurfaceSize) -> Self {
        let width = u64::from(size.buffer.width);
        let height = u64::from(size.buffer.height);
        if width <= MAX_WIDTH && height <= MAX_HEIGHT {
            return Self {
                buffer: size.buffer,
                scale: size.render_scale,
                scaled: false,
            };
        }

        let (limit, dimension) = if width * MAX_HEIGHT >= height * MAX_WIDTH {
            (MAX_WIDTH, width)
        } else {
            (MAX_HEIGHT, height)
        };
        let fit = |pixels: u64| ((pixels * limit) / dimension).max(1) as u32;
        let units = (u64::from(size.render_scale.units()) * limit + dimension / 2) / dimension;
        Self {
            buffer: FrameSize::new(fit(width), fit(height)),
            scale: RenderScale::from_units(units.max(1) as u32),
            scaled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_smaller_buffers_and_fits_larger_outputs_inside_the_video_cap() {
        for (logical, scale, native, video) in [
            ((1920, 1080), None, (1920, 1080), (1920, 1080)),
            ((2560, 1440), None, (2560, 1440), (1920, 1080)),
            ((2560, 1440), Some(180), (3840, 2160), (1920, 1080)),
            ((3440, 1440), None, (3440, 1440), (1920, 803)),
            ((2160, 3840), None, (2160, 3840), (607, 1080)),
        ] {
            let size = SurfaceSize::new_with_fractional_scale(logical.0, logical.1, 1, scale);
            let geometry = VideoGeometry::for_surface(size);
            assert_eq!(size.buffer, FrameSize::new(native.0, native.1));
            assert_eq!(geometry.buffer, FrameSize::new(video.0, video.1));
            assert_eq!(geometry.scaled, native != video);
            assert!(geometry.scale.units() <= size.render_scale.units());
        }
    }
}
