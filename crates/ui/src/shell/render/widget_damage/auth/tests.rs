use super::*;
use veila_renderer::{FrameSize, shape::Rect};

fn isolated() -> WidgetRegions {
    WidgetRegions {
        size: FrameSize::new(800, 600),
        header: Some(Rect::new(20, 20, 100, 40)),
        media: None,
        indicators: None,
        auth: Some(Rect::new(200, 300, 400, 80)),
        weather: None,
        partial_unsafe: false,
    }
}

#[test]
fn shrinking_auth_damage_retains_old_status_bounds() {
    let previous = isolated();
    let current = WidgetRegions {
        auth: Some(Rect::new(250, 310, 200, 40)),
        ..previous
    };
    assert_eq!(
        current.auth_damage_since(previous),
        WidgetDamage::Region(previous.auth.unwrap())
    );
}

#[test]
fn disappearing_auth_retains_previous_damage() {
    let previous = isolated();
    assert_eq!(
        WidgetRegions {
            auth: None,
            ..previous
        }
        .auth_damage_since(previous),
        WidgetDamage::Region(previous.auth.unwrap())
    );
}

#[test]
fn every_dynamic_neighbor_blocks_auth_partial_repainting() {
    let previous = isolated();
    let overlap = Some(Rect::new(220, 310, 40, 40));
    for regions in [
        WidgetRegions {
            header: overlap,
            ..previous
        },
        WidgetRegions {
            media: overlap,
            ..previous
        },
        WidgetRegions {
            indicators: overlap,
            ..previous
        },
        WidgetRegions {
            weather: overlap,
            ..previous
        },
    ] {
        assert_eq!(regions.auth_damage_since(regions), WidgetDamage::Full);
    }
}

#[test]
fn old_auth_bounds_and_neighbor_paint_padding_block_partial_repainting() {
    let previous = WidgetRegions {
        header: Some(Rect::new(190, 275, 20, 14)),
        ..isolated()
    };
    let current = WidgetRegions {
        auth: Some(Rect::new(250, 310, 200, 40)),
        ..previous
    };
    assert_eq!(current.auth_damage_since(previous), WidgetDamage::Full);
}

#[test]
fn changed_neighbor_requires_full_repainting_even_when_separate() {
    let previous = isolated();
    for header in [None, Some(Rect::new(20, 20, 110, 40))] {
        assert_eq!(
            WidgetRegions { header, ..previous }.auth_damage_since(previous),
            WidgetDamage::Full
        );
    }
}

#[test]
fn unsafe_previous_or_current_scene_requires_full_repainting() {
    let safe = isolated();
    let unsafe_scene = WidgetRegions {
        partial_unsafe: true,
        ..safe
    };
    assert_eq!(safe.auth_damage_since(unsafe_scene), WidgetDamage::Full);
    assert_eq!(unsafe_scene.auth_damage_since(safe), WidgetDamage::Full);
}

#[test]
fn changed_frame_or_missing_auth_geometry_requires_full_repainting() {
    let previous = isolated();
    let missing = WidgetRegions {
        auth: None,
        ..previous
    };
    assert_eq!(missing.auth_damage_since(missing), WidgetDamage::Full);
    let empty = WidgetRegions {
        auth: Some(Rect::new(0, 0, 0, 0)),
        ..missing
    };
    assert_eq!(empty.auth_damage_since(empty), WidgetDamage::Full);
    assert_eq!(
        WidgetRegions {
            size: FrameSize::new(960, 640),
            ..previous
        }
        .auth_damage_since(previous),
        WidgetDamage::Full
    );
}

#[cfg(test)]
mod pixels;
