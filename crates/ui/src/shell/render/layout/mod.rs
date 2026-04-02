mod anchors;
mod layer;
mod metrics;
mod types;

#[cfg(test)]
mod tests;

pub(super) use anchors::{hero_block_x, role_anchors_with_groups, top_role_top};
pub(super) use layer::{layer_center_x, layer_rect};
pub(crate) use types::SceneMetrics;
pub(super) use types::{
    AnchorOffsets, AuthGroupHeights, FooterHeights, InputPlacement, LayerPlacement,
    RoleAnchorInput, RoleAnchors,
};

#[cfg(test)]
pub(super) use anchors::role_anchors;

fn horizontal_auth_padding(frame_width: i32) -> i32 {
    ((frame_width / 24).clamp(24, 72)).max(0)
}
