use std::sync::Arc;

use image::{Rgba, RgbaImage};

use super::{BackgroundAsset, BackgroundKind, BackgroundScaling, BackgroundTreatment};
use crate::{ClearColor, FrameSize};

#[test]
fn wallpaper_strengths_above_twelve_progressively_spread_broad_edges() {
    let image = Arc::new(RgbaImage::from_fn(1024, 16, |x, _| {
        let value = if x < 512 { 0 } else { 255 };
        Rgba([value, value, value, 255])
    }));
    for scaling in [
        BackgroundScaling::Fill,
        BackgroundScaling::Fit,
        BackgroundScaling::Center,
        BackgroundScaling::Tile,
        BackgroundScaling::Stretch,
    ] {
        let mut previous = 0;
        for strength in [12, 24, 32, 64, 128, 255] {
            let asset = BackgroundAsset {
                kind: BackgroundKind::Image {
                    image: Arc::clone(&image),
                    fallback: ClearColor::opaque(0, 0, 0),
                },
                treatment: BackgroundTreatment {
                    blur_radius: strength,
                    scaling,
                    ..BackgroundTreatment::default()
                },
            };
            let buffer = asset.render(FrameSize::new(1024, 16)).expect("render");
            let value = buffer.pixels()[(8 * 1024 + 480) * 4];
            assert!(
                value > previous,
                "{scaling:?}, {strength}: {value} <= {previous}"
            );
            assert!(
                buffer
                    .pixels()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .all(|pixel| pixel[3] == 255)
            );
            previous = value;
        }
    }
}
