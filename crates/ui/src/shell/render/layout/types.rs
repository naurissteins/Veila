use veila_common::{ClockAlignment, LayerAlignment, LayerVerticalAlignment};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SceneMetrics {
    pub center_x: i32,
    pub auth_center_x: i32,
    pub content_width: u32,
    pub clock_width: u32,
    pub input_width: i32,
    pub input_height: i32,
    pub avatar_size: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleAnchors {
    pub identity_y: Option<i32>,
    pub hero_y: i32,
    pub auth_y: i32,
    pub footer_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AnchorOffsets {
    pub clock_alignment: ClockAlignment,
    pub clock_offset_y: Option<i32>,
    pub weather_bottom_padding: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FooterHeights {
    pub render: i32,
    pub clearance: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthGroupHeights {
    pub identity: i32,
    pub input_anchor: i32,
    pub input_render: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleAnchorInput {
    pub frame_height: i32,
    pub hero_height: i32,
    pub auth_anchor_height: i32,
    pub auth_render_height: i32,
    pub auth_groups: AuthGroupHeights,
    pub footer_heights: FooterHeights,
    pub offsets: AnchorOffsets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerPlacement {
    pub alignment: LayerAlignment,
    pub full_width: bool,
    pub width: Option<i32>,
    pub full_height: bool,
    pub height: Option<i32>,
    pub vertical_alignment: LayerVerticalAlignment,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
    pub left_padding: Option<i32>,
    pub right_padding: Option<i32>,
    pub top_padding: Option<i32>,
    pub bottom_padding: Option<i32>,
}

impl FooterHeights {
    #[cfg(test)]
    pub const fn same(height: i32) -> Self {
        Self {
            render: height,
            clearance: height,
        }
    }
}
