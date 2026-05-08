mod anchors;
mod metrics;
mod types;

#[cfg(test)]
mod tests;

pub(super) use anchors::{
    anchored_block_x, anchored_block_y, hero_block_x, role_anchors_with_groups,
};
pub(crate) use types::SceneMetrics;
pub(super) use types::{
    AnchorOffsets, AuthGroupHeights, FooterHeights, RoleAnchorInput, RoleAnchors,
};

#[cfg(test)]
pub(super) use anchors::role_anchors;

fn horizontal_auth_padding(frame_width: i32) -> i32 {
    ((frame_width / 24).clamp(24, 72)).max(0)
}
