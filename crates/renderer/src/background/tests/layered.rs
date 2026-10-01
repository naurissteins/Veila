use crate::background::{
    BackgroundAsset, BackgroundLayered, BackgroundLayeredBase, BackgroundLayeredBlob,
    GeneratedBackground,
};
use crate::{ClearColor, FrameSize, SoftwareBuffer};

#[test]
fn blob_preserves_smoothstep_falloff_through_its_boundary() {
    let buffer = render(BackgroundLayeredBlob {
        color: ClearColor::rgba(255, 0, 0, 128),
        x: 50,
        y: 50,
        size: 50,
    });
    let center_row = &buffer.pixels()[4 * 9 * 4..5 * 9 * 4];

    assert_eq!(
        center_row,
        &[
            30, 20, 10, 255, 28, 18, 29, 255, 22, 15, 71, 255, 17, 12, 114, 255, 15, 10, 133, 255,
            17, 12, 114, 255, 22, 15, 71, 255, 28, 18, 29, 255, 30, 20, 10, 255,
        ]
    );
}

#[test]
fn clipped_blob_keeps_boundary_and_exterior_pixels_unchanged() {
    let buffer = render(BackgroundLayeredBlob {
        color: ClearColor::opaque(255, 0, 0),
        x: 0,
        y: 0,
        size: 25,
    });

    assert_eq!(&buffer.pixels()[..4], &[0, 0, 255, 255]);
    for (x, y) in [(2, 0), (0, 2), (2, 2), (8, 8)] {
        let offset = (y * 9 + x) * 4;
        assert_eq!(&buffer.pixels()[offset..offset + 4], &[30, 20, 10, 255]);
    }
}

fn render(blob: BackgroundLayeredBlob) -> SoftwareBuffer {
    BackgroundAsset::load(
        None,
        ClearColor::opaque(0, 0, 0),
        Some(GeneratedBackground::Layered(BackgroundLayered {
            base: BackgroundLayeredBase::Solid(ClearColor::opaque(10, 20, 30)),
            blobs: [Some(blob), None, None],
        })),
        Default::default(),
    )
    .unwrap()
    .render(FrameSize::new(9, 9))
    .unwrap()
}
