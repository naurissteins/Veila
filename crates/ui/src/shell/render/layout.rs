use veila_common::{
    CenterStackStyle, ClockAlignment, InputAlignment, LayerAlignment, LayerVerticalAlignment,
};
use veila_renderer::shape::Rect;

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
pub(super) struct InputPlacement {
    pub alignment: InputAlignment,
    pub center_in_layer: bool,
    pub layer_center_x: Option<i32>,
    pub horizontal_padding: Option<i32>,
    pub offset_x: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RoleAnchors {
    pub identity_y: Option<i32>,
    pub hero_y: i32,
    pub auth_y: i32,
    pub footer_y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct AnchorOffsets {
    pub auth_stack: Option<i32>,
    pub input_vertical_padding: Option<i32>,
    pub input_offset_y: Option<i32>,
    pub header_top: Option<i32>,
    pub center_stack_style: CenterStackStyle,
    pub clock_alignment: ClockAlignment,
    pub clock_offset_y: Option<i32>,
    pub weather_bottom_padding: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FooterHeights {
    pub render: i32,
    pub clearance: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AuthGroupHeights {
    pub identity: i32,
    pub input_anchor: i32,
    pub input_render: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RoleAnchorInput {
    pub frame_height: i32,
    pub hero_height: i32,
    pub auth_anchor_height: i32,
    pub auth_render_height: i32,
    pub auth_groups: AuthGroupHeights,
    pub footer_heights: FooterHeights,
    pub input_alignment: InputAlignment,
    pub offsets: AnchorOffsets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LayerPlacement {
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
    const fn same(height: i32) -> Self {
        Self {
            render: height,
            clearance: height,
        }
    }
}

impl SceneMetrics {
    #[cfg(test)]
    pub(super) fn from_frame(
        width: i32,
        height: i32,
        configured_input_width: Option<i32>,
        configured_input_height: Option<i32>,
        configured_avatar_size: Option<i32>,
        input_alignment: InputAlignment,
    ) -> Self {
        Self::from_frame_with_input_placement(
            width,
            height,
            configured_input_width,
            configured_input_height,
            configured_avatar_size,
            InputPlacement {
                alignment: input_alignment,
                center_in_layer: false,
                layer_center_x: None,
                horizontal_padding: None,
                offset_x: None,
            },
        )
    }

    pub(super) fn from_frame_with_input_placement(
        width: i32,
        height: i32,
        configured_input_width: Option<i32>,
        configured_input_height: Option<i32>,
        configured_avatar_size: Option<i32>,
        input_placement: InputPlacement,
    ) -> Self {
        let scene_center_x = width / 2;
        let scene_width = ((width as f32) * 0.34) as i32;
        let input_width = configured_input_width
            .unwrap_or_else(|| (((scene_width as f32) * 0.7) as i32).clamp(220, 320))
            .clamp(180, 560);
        let input_height = configured_input_height
            .unwrap_or_else(|| (((height as f32) * 0.072) as i32).clamp(48, 58))
            .clamp(40, 96);
        let avatar_size = configured_avatar_size
            .unwrap_or_else(|| (width.min(height) / 7).clamp(84, 108))
            .clamp(56, 160);
        let horizontal_padding = input_placement
            .horizontal_padding
            .unwrap_or_else(|| horizontal_auth_padding(width));
        let base_auth_center_x = if input_placement.center_in_layer {
            input_placement.layer_center_x.unwrap_or_else(|| {
                auth_center_x(
                    width,
                    input_width,
                    horizontal_padding,
                    input_placement.alignment,
                )
            })
        } else {
            auth_center_x(
                width,
                input_width,
                horizontal_padding,
                input_placement.alignment,
            )
        };
        let auth_center_x = apply_auth_offset_x(
            base_auth_center_x,
            width,
            input_width,
            input_placement.offset_x,
        );

        Self {
            center_x: scene_center_x,
            auth_center_x,
            content_width: (input_width + 72).max(220) as u32,
            clock_width: (input_width + 140).max(280) as u32,
            input_width,
            input_height,
            avatar_size,
        }
    }

    pub(super) fn input_rect(self, y: i32) -> Rect {
        Rect::new(
            self.auth_center_x - self.input_width / 2,
            y,
            self.input_width,
            self.input_height,
        )
    }
}

#[cfg(test)]
pub(super) fn role_anchors(
    frame_height: i32,
    hero_height: i32,
    auth_anchor_height: i32,
    auth_render_height: i32,
    footer_heights: FooterHeights,
    input_alignment: InputAlignment,
    offsets: AnchorOffsets,
) -> RoleAnchors {
    role_anchors_with_groups(RoleAnchorInput {
        frame_height,
        hero_height,
        auth_anchor_height,
        auth_render_height,
        auth_groups: AuthGroupHeights {
            identity: 0,
            input_anchor: auth_anchor_height,
            input_render: auth_render_height,
        },
        footer_heights,
        input_alignment,
        offsets,
    })
}

pub(super) fn role_anchors_with_groups(input: RoleAnchorInput) -> RoleAnchors {
    let frame_height = input.frame_height;
    let hero_height = input.hero_height;
    let auth_anchor_height = input.auth_anchor_height;
    let auth_render_height = input.auth_render_height;
    let auth_groups = input.auth_groups;
    let footer_heights = input.footer_heights;
    let input_alignment = input.input_alignment;
    let offsets = input.offsets;
    let identity_height = auth_groups.identity;
    let input_anchor_height = auth_groups.input_anchor;
    let input_render_height = auth_groups.input_render;
    let hero_top = top_role_top(frame_height, offsets.header_top);
    let hero_y = match offsets.clock_alignment {
        ClockAlignment::TopCenter => hero_top,
        ClockAlignment::TopRight => hero_top,
        ClockAlignment::TopLeft => hero_top,
        ClockAlignment::CenterCenter => centered_role_top(frame_height, hero_height, 0.5),
    } + offsets.clock_offset_y.unwrap_or(0);
    let footer_y = frame_height
        - footer_heights.render
        - offsets.weather_bottom_padding.unwrap_or(48).clamp(0, 512);
    let hero_bottom = hero_y + hero_height;
    let minimum_gap = if hero_height > 0 && auth_anchor_height > 0 {
        18
    } else {
        0
    };
    let auth_offset = offsets.auth_stack.unwrap_or(0);
    let vertical_padding = offsets.input_vertical_padding.unwrap_or(0).clamp(0, 512);
    let input_offset_y = offsets.input_offset_y.unwrap_or(0);
    let top_auth_y = vertical_padding.max(hero_bottom + minimum_gap);
    let centered_auth_y = centered_role_top(frame_height, auth_anchor_height, 0.5);
    let auth_footer_y = frame_height
        - footer_heights.clearance
        - offsets.weather_bottom_padding.unwrap_or(48).clamp(0, 512);
    let bottom_auth_y = (frame_height - vertical_padding - auth_render_height)
        .min(auth_footer_y - auth_render_height - 24);
    let min_auth_y = hero_bottom + minimum_gap;
    let max_auth_y = auth_footer_y - auth_render_height - 24;

    if matches!(offsets.clock_alignment, ClockAlignment::CenterCenter)
        && matches!(input_alignment, InputAlignment::CenterCenter)
        && hero_height > 0
        && auth_anchor_height > 0
    {
        let combined_height = hero_height + minimum_gap + auth_anchor_height;
        let group_shift = offsets.clock_offset_y.unwrap_or(0);

        return match offsets.center_stack_style {
            CenterStackStyle::HeroAuth => {
                let centered_hero_y = centered_role_top(frame_height, combined_height, 0.5).clamp(
                    hero_top,
                    (max_auth_y - hero_height - minimum_gap).max(hero_top),
                ) + group_shift;
                let auth_y =
                    (centered_hero_y + hero_height + minimum_gap + auth_offset + input_offset_y)
                        .clamp(centered_hero_y + hero_height + minimum_gap, max_auth_y);

                RoleAnchors {
                    identity_y: None,
                    hero_y: centered_hero_y,
                    auth_y,
                    footer_y,
                }
            }
            CenterStackStyle::AuthHero => {
                let max_group_top =
                    (auth_footer_y - auth_anchor_height - minimum_gap - hero_height - 24).max(0);
                let centered_auth_y =
                    centered_role_top(frame_height, combined_height, 0.5).clamp(0, max_group_top);
                let auth_y = (centered_auth_y + group_shift + auth_offset + input_offset_y)
                    .clamp(0, max_group_top);

                RoleAnchors {
                    identity_y: None,
                    hero_y: auth_y + auth_anchor_height + minimum_gap,
                    auth_y,
                    footer_y,
                }
            }
            CenterStackStyle::IdentityHeroInput
                if identity_height > 0 && input_anchor_height > 0 =>
            {
                let identity_gap = if identity_height > 0 && hero_height > 0 {
                    18
                } else {
                    0
                };
                let input_gap = if hero_height > 0 && input_anchor_height > 0 {
                    18
                } else {
                    0
                };
                let combined_height =
                    identity_height + identity_gap + hero_height + input_gap + input_anchor_height;
                let max_identity_y = (auth_footer_y
                    - input_render_height
                    - 24
                    - input_gap
                    - hero_height
                    - identity_gap
                    - identity_height)
                    .max(0);
                let identity_y = (centered_role_top(frame_height, combined_height, 0.5)
                    + group_shift)
                    .clamp(0, max_identity_y);
                let hero_y = identity_y + identity_height + identity_gap;
                let max_input_y = auth_footer_y - input_render_height - 24;
                let auth_y = (hero_y + hero_height + input_gap + auth_offset + input_offset_y)
                    .clamp(hero_y + hero_height + input_gap, max_input_y);

                RoleAnchors {
                    identity_y: Some(identity_y),
                    hero_y,
                    auth_y,
                    footer_y,
                }
            }
            CenterStackStyle::IdentityHeroInput => {
                let centered_hero_y = centered_role_top(frame_height, combined_height, 0.5).clamp(
                    hero_top,
                    (max_auth_y - hero_height - minimum_gap).max(hero_top),
                ) + group_shift;
                let auth_y =
                    (centered_hero_y + hero_height + minimum_gap + auth_offset + input_offset_y)
                        .clamp(centered_hero_y + hero_height + minimum_gap, max_auth_y);

                RoleAnchors {
                    identity_y: None,
                    hero_y: centered_hero_y,
                    auth_y,
                    footer_y,
                }
            }
        };
    }

    if max_auth_y < min_auth_y {
        let combined_height = hero_height + minimum_gap + auth_render_height;
        let combined_top = ((frame_height - combined_height) / 2).max(hero_top);

        return RoleAnchors {
            identity_y: None,
            hero_y: combined_top,
            auth_y: combined_top + hero_height + minimum_gap,
            footer_y,
        };
    }

    let auth_y = (match input_alignment {
        InputAlignment::TopCenter | InputAlignment::TopRight | InputAlignment::TopLeft => {
            top_auth_y
        }
        InputAlignment::BottomCenter | InputAlignment::BottomRight | InputAlignment::BottomLeft => {
            bottom_auth_y
        }
        InputAlignment::CenterCenter | InputAlignment::CenterRight | InputAlignment::CenterLeft => {
            centered_auth_y
        }
    } + auth_offset
        + input_offset_y)
        .clamp(min_auth_y, max_auth_y);

    RoleAnchors {
        identity_y: None,
        hero_y,
        auth_y,
        footer_y,
    }
}

fn horizontal_auth_padding(frame_width: i32) -> i32 {
    ((frame_width / 24).clamp(24, 72)).max(0)
}

fn auth_center_x(
    frame_width: i32,
    input_width: i32,
    horizontal_padding: i32,
    input_alignment: InputAlignment,
) -> i32 {
    let centered = frame_width / 2;
    let left = (horizontal_padding + input_width / 2).clamp(input_width / 2, frame_width);
    let right =
        (frame_width - horizontal_padding - input_width / 2).clamp(input_width / 2, frame_width);
    match input_alignment {
        InputAlignment::TopLeft | InputAlignment::CenterLeft | InputAlignment::BottomLeft => left,
        InputAlignment::TopRight | InputAlignment::CenterRight | InputAlignment::BottomRight => {
            right
        }
        InputAlignment::TopCenter | InputAlignment::CenterCenter | InputAlignment::BottomCenter => {
            centered
        }
    }
}

fn apply_auth_offset_x(
    auth_center_x: i32,
    frame_width: i32,
    input_width: i32,
    input_offset_x: Option<i32>,
) -> i32 {
    let min_x = input_width / 2;
    let max_x = (frame_width - input_width / 2).max(min_x);
    (auth_center_x + input_offset_x.unwrap_or(0)).clamp(min_x, max_x)
}

fn centered_role_top(frame_height: i32, role_height: i32, center_factor: f32) -> i32 {
    ((frame_height as f32) * center_factor) as i32 - role_height / 2
}

pub(super) fn hero_block_x(
    frame_width: i32,
    block_width: i32,
    alignment: ClockAlignment,
    center_x_override: Option<i32>,
    offset_x: Option<i32>,
) -> i32 {
    let base_x = match center_x_override {
        Some(center_x) => center_x - block_width / 2,
        None => match alignment {
            ClockAlignment::TopCenter | ClockAlignment::CenterCenter => {
                frame_width / 2 - block_width / 2
            }
            ClockAlignment::TopLeft => horizontal_auth_padding(frame_width),
            ClockAlignment::TopRight => {
                (frame_width - horizontal_auth_padding(frame_width) - block_width).max(0)
            }
        },
    };

    (base_x + offset_x.unwrap_or(0)).clamp(0, (frame_width - block_width).max(0))
}

pub(super) fn layer_rect(frame_width: i32, frame_height: i32, placement: LayerPlacement) -> Rect {
    let left_padding = placement
        .left_padding
        .unwrap_or(0)
        .clamp(0, frame_width.max(0));
    let right_padding = placement
        .right_padding
        .unwrap_or(0)
        .clamp(0, frame_width.max(0));
    let top_padding = placement
        .top_padding
        .unwrap_or(0)
        .clamp(0, frame_height.max(0));
    let bottom_padding = placement
        .bottom_padding
        .unwrap_or(0)
        .clamp(0, frame_height.max(0));
    let safe_left = left_padding;
    let safe_right = (frame_width - right_padding).max(safe_left + 1);
    let safe_width = (safe_right - safe_left).max(1);
    let safe_top = top_padding.min(frame_height.saturating_sub(1));
    let safe_bottom = (frame_height - bottom_padding).max(safe_top + 1);
    let safe_height = (safe_bottom - safe_top).max(1);
    let width = if placement.full_width {
        safe_width
    } else {
        placement
            .width
            .unwrap_or((frame_width as f32 * 0.36) as i32)
            .clamp(1, safe_width)
    };
    let height = if placement.full_height {
        safe_height
    } else {
        placement
            .height
            .unwrap_or(safe_height)
            .clamp(1, safe_height)
    };
    let offset_x = placement.offset_x.unwrap_or(0);
    let offset_y = placement.offset_y.unwrap_or(0);
    let unclamped_x = match placement.alignment {
        LayerAlignment::Left => safe_left + offset_x,
        LayerAlignment::Center => safe_left + (safe_width - width) / 2 + offset_x,
        LayerAlignment::Right => safe_right - width + offset_x,
    };
    let x = unclamped_x.clamp(safe_left - width + 1, safe_right - 1);
    let unclamped_y = match placement.vertical_alignment {
        LayerVerticalAlignment::Top => safe_top,
        LayerVerticalAlignment::Center => safe_top + (safe_height - height) / 2,
        LayerVerticalAlignment::Bottom => safe_bottom - height,
    } + offset_y;
    let y = unclamped_y.clamp(safe_top, safe_bottom - height);

    Rect::new(x, y, width, height)
}

pub(super) fn layer_center_x(frame_width: i32, placement: LayerPlacement) -> i32 {
    let rect = layer_rect(frame_width, 1, placement);
    rect.x + rect.width / 2
}

pub(super) fn top_role_top(frame_height: i32, header_top_offset: Option<i32>) -> i32 {
    ((frame_height / 14).clamp(28, 72) + header_top_offset.unwrap_or(0)).max(0)
}

#[cfg(test)]
mod tests {
    use veila_common::{
        CenterStackStyle, ClockAlignment, InputAlignment, LayerAlignment, LayerVerticalAlignment,
    };

    use super::{
        AnchorOffsets, AuthGroupHeights, FooterHeights, InputPlacement, LayerPlacement,
        RoleAnchorInput, SceneMetrics, hero_block_x, layer_center_x, layer_rect, role_anchors,
        role_anchors_with_groups,
    };

    #[test]
    fn falls_back_to_stacked_roles_when_they_would_overlap() {
        let anchors = role_anchors(
            400,
            160,
            170,
            170,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets::default(),
        );

        assert_eq!(anchors.hero_y, 28);
        assert_eq!(anchors.auth_y, 206);
    }

    #[test]
    fn uses_slimmer_input_height() {
        let metrics =
            SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);

        assert_eq!(metrics.input_height, 51);
    }

    #[test]
    fn uses_narrower_input_width() {
        let metrics =
            SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);

        assert_eq!(metrics.input_width, 304);
    }

    #[test]
    fn uses_smaller_avatar_size_for_compact_hero_stack() {
        let metrics =
            SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);

        assert_eq!(metrics.avatar_size, 102);
    }

    #[test]
    fn uses_configured_avatar_size_when_present() {
        let metrics = SceneMetrics::from_frame(
            1280,
            720,
            None,
            None,
            Some(88),
            InputAlignment::CenterCenter,
        );

        assert_eq!(metrics.avatar_size, 88);
    }

    #[test]
    fn uses_configured_input_width_when_present() {
        let metrics = SceneMetrics::from_frame(
            1280,
            720,
            Some(280),
            None,
            None,
            InputAlignment::CenterCenter,
        );

        assert_eq!(metrics.input_width, 280);
    }

    #[test]
    fn uses_configured_input_height_when_present() {
        let metrics = SceneMetrics::from_frame(
            1280,
            720,
            None,
            Some(54),
            None,
            InputAlignment::CenterCenter,
        );

        assert_eq!(metrics.input_height, 54);
    }

    #[test]
    fn keeps_auth_close_to_hero_when_space_allows() {
        let anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets::default(),
        );

        assert_eq!(anchors.hero_y, 51);
        assert_eq!(anchors.auth_y, 262);
    }

    #[test]
    fn keeps_auth_anchor_stable_when_status_height_grows() {
        let without_status = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets::default(),
        );
        let with_status = role_anchors(
            720,
            54,
            197,
            235,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets::default(),
        );

        assert_eq!(without_status.auth_y, 262);
        assert_eq!(with_status.auth_y, 262);
    }

    #[test]
    fn applies_configured_header_top_offset() {
        let default_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets::default(),
        );
        let shifted_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets {
                header_top: Some(-12),
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(default_anchors.hero_y, 51);
        assert_eq!(shifted_anchors.hero_y, 39);
    }

    #[test]
    fn supports_centered_clock_alignment() {
        let default_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets::default(),
        );
        let centered_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets {
                clock_alignment: ClockAlignment::CenterCenter,
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(default_anchors.hero_y, 51);
        assert_eq!(centered_anchors.hero_y, 226);
        assert_eq!(centered_anchors.auth_y, 298);
    }

    #[test]
    fn keeps_centered_clock_and_auth_visually_grouped() {
        let without_status = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets {
                clock_alignment: ClockAlignment::CenterCenter,
                ..AnchorOffsets::default()
            },
        );
        let with_status = role_anchors(
            720,
            54,
            197,
            235,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets {
                clock_alignment: ClockAlignment::CenterCenter,
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(without_status.hero_y, 226);
        assert_eq!(without_status.auth_y, 298);
        assert_eq!(with_status.hero_y, 226);
        assert_eq!(with_status.auth_y, 298);
    }

    #[test]
    fn supports_auth_hero_order_for_centered_grouped_layouts() {
        let anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets {
                center_stack_style: CenterStackStyle::AuthHero,
                clock_alignment: ClockAlignment::CenterCenter,
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(anchors.auth_y, 226);
        assert_eq!(anchors.hero_y, 441);
        assert!(anchors.auth_y < anchors.hero_y);
    }

    #[test]
    fn supports_identity_hero_input_style_for_centered_grouped_layouts() {
        let anchors = role_anchors_with_groups(RoleAnchorInput {
            frame_height: 720,
            hero_height: 54,
            auth_anchor_height: 197,
            auth_render_height: 197,
            auth_groups: AuthGroupHeights {
                identity: 72,
                input_anchor: 51,
                input_render: 51,
            },
            footer_heights: FooterHeights::same(0),
            input_alignment: InputAlignment::CenterCenter,
            offsets: AnchorOffsets {
                center_stack_style: CenterStackStyle::IdentityHeroInput,
                clock_alignment: ClockAlignment::CenterCenter,
                ..AnchorOffsets::default()
            },
        });

        assert_eq!(anchors.identity_y, Some(254));
        assert_eq!(anchors.hero_y, 344);
        assert_eq!(anchors.auth_y, 416);
        assert!(
            anchors
                .identity_y
                .is_some_and(|identity_y| identity_y < anchors.hero_y)
        );
        assert!(anchors.hero_y < anchors.auth_y);
    }

    #[test]
    fn supports_top_side_clock_alignment_positions() {
        assert_eq!(
            hero_block_x(1280, 300, ClockAlignment::TopLeft, None, None),
            53
        );
        assert_eq!(
            hero_block_x(1280, 300, ClockAlignment::TopRight, None, None),
            927
        );
    }

    #[test]
    fn applies_clock_horizontal_offset() {
        assert_eq!(
            hero_block_x(1280, 300, ClockAlignment::TopCenter, None, Some(24)),
            514
        );
        assert_eq!(
            hero_block_x(1280, 300, ClockAlignment::TopRight, None, Some(-20)),
            907
        );
    }

    #[test]
    fn centers_clock_block_inside_layer_when_requested() {
        assert_eq!(
            hero_block_x(1280, 300, ClockAlignment::TopRight, Some(1000), None),
            850
        );
    }

    #[test]
    fn applies_clock_vertical_offset() {
        let anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets {
                clock_offset_y: Some(18),
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(anchors.hero_y, 69);
    }

    #[test]
    fn applies_configured_auth_stack_offset() {
        let default_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets::default(),
        );
        let shifted_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::CenterCenter,
            AnchorOffsets {
                auth_stack: Some(16),
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(default_anchors.auth_y, 262);
        assert_eq!(shifted_anchors.auth_y, 278);
    }

    #[test]
    fn applies_configured_weather_bottom_padding() {
        let default_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(80),
            InputAlignment::CenterCenter,
            AnchorOffsets::default(),
        );
        let shifted_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(80),
            InputAlignment::CenterCenter,
            AnchorOffsets {
                weather_bottom_padding: Some(72),
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(default_anchors.footer_y, 592);
        assert_eq!(shifted_anchors.footer_y, 568);
    }

    #[test]
    fn places_auth_center_x_on_left_and_right_edges() {
        let left =
            SceneMetrics::from_frame(1280, 720, Some(300), None, None, InputAlignment::CenterLeft);
        let right = SceneMetrics::from_frame(
            1280,
            720,
            Some(300),
            None,
            None,
            InputAlignment::CenterRight,
        );

        assert!(left.auth_center_x < left.center_x);
        assert!(right.auth_center_x > right.center_x);
        assert_eq!(left.input_rect(100).x, 53);
        assert_eq!(right.input_rect(100).x, 927);
    }

    #[test]
    fn applies_configured_input_offset_x() {
        let default_metrics =
            SceneMetrics::from_frame(1280, 720, Some(300), None, None, InputAlignment::TopRight);
        let shifted_metrics = SceneMetrics::from_frame_with_input_placement(
            1280,
            720,
            Some(300),
            None,
            None,
            InputPlacement {
                alignment: InputAlignment::TopRight,
                center_in_layer: false,
                layer_center_x: None,
                horizontal_padding: None,
                offset_x: Some(-36),
            },
        );

        assert_eq!(default_metrics.auth_center_x, 1077);
        assert_eq!(shifted_metrics.auth_center_x, 1041);
    }

    #[test]
    fn applies_configured_input_horizontal_padding() {
        let default_metrics =
            SceneMetrics::from_frame(1280, 720, Some(300), None, None, InputAlignment::CenterLeft);
        let shifted_metrics = SceneMetrics::from_frame_with_input_placement(
            1280,
            720,
            Some(300),
            None,
            None,
            InputPlacement {
                alignment: InputAlignment::CenterLeft,
                center_in_layer: false,
                layer_center_x: None,
                horizontal_padding: Some(96),
                offset_x: None,
            },
        );

        assert_eq!(default_metrics.auth_center_x, 203);
        assert_eq!(shifted_metrics.auth_center_x, 246);
    }

    #[test]
    fn centers_auth_block_inside_layer_when_requested() {
        let metrics = SceneMetrics::from_frame_with_input_placement(
            1280,
            720,
            Some(300),
            None,
            None,
            InputPlacement {
                alignment: InputAlignment::BottomRight,
                center_in_layer: true,
                layer_center_x: Some(980),
                horizontal_padding: None,
                offset_x: None,
            },
        );

        assert_eq!(metrics.auth_center_x, 980);
        assert_eq!(metrics.input_rect(100).x, 830);
    }

    #[test]
    fn computes_layer_center_from_layer_rect() {
        let rect = layer_rect(
            1280,
            720,
            LayerPlacement {
                alignment: LayerAlignment::Right,
                full_width: false,
                width: Some(520),
                full_height: false,
                height: Some(420),
                vertical_alignment: LayerVerticalAlignment::Top,
                offset_x: Some(-12),
                offset_y: Some(0),
                left_padding: Some(24),
                right_padding: Some(36),
                top_padding: Some(18),
                bottom_padding: Some(22),
            },
        );

        assert_eq!(rect.x, 712);
        assert_eq!(rect.y, 18);
        assert_eq!(rect.height, 420);
        assert_eq!(
            layer_center_x(
                1280,
                LayerPlacement {
                    alignment: LayerAlignment::Right,
                    full_width: false,
                    width: Some(520),
                    full_height: false,
                    height: Some(420),
                    vertical_alignment: LayerVerticalAlignment::Top,
                    offset_x: Some(-12),
                    offset_y: Some(0),
                    left_padding: Some(24),
                    right_padding: Some(36),
                    top_padding: Some(18),
                    bottom_padding: Some(22),
                },
            ),
            972
        );
    }

    #[test]
    fn supports_configured_layer_vertical_alignment() {
        let center_rect = layer_rect(
            1280,
            720,
            LayerPlacement {
                alignment: LayerAlignment::Center,
                full_width: false,
                width: Some(520),
                full_height: false,
                height: Some(420),
                vertical_alignment: LayerVerticalAlignment::Center,
                offset_x: None,
                offset_y: Some(0),
                left_padding: Some(24),
                right_padding: Some(36),
                top_padding: Some(18),
                bottom_padding: Some(22),
            },
        );
        let bottom_rect = layer_rect(
            1280,
            720,
            LayerPlacement {
                alignment: LayerAlignment::Center,
                full_width: false,
                width: Some(520),
                full_height: false,
                height: Some(420),
                vertical_alignment: LayerVerticalAlignment::Bottom,
                offset_x: None,
                offset_y: Some(0),
                left_padding: Some(24),
                right_padding: Some(36),
                top_padding: Some(18),
                bottom_padding: Some(22),
            },
        );

        assert_eq!(center_rect.y, 148);
        assert_eq!(bottom_rect.y, 278);
    }

    #[test]
    fn applies_configured_layer_offset_y() {
        let rect = layer_rect(
            1280,
            720,
            LayerPlacement {
                alignment: LayerAlignment::Center,
                full_width: false,
                width: Some(520),
                full_height: false,
                height: Some(420),
                vertical_alignment: LayerVerticalAlignment::Center,
                offset_x: None,
                offset_y: Some(24),
                left_padding: Some(24),
                right_padding: Some(36),
                top_padding: Some(18),
                bottom_padding: Some(22),
            },
        );

        assert_eq!(rect.y, 172);
    }

    #[test]
    fn applies_configured_input_offset_y() {
        let default_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::TopCenter,
            AnchorOffsets::default(),
        );
        let shifted_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::TopCenter,
            AnchorOffsets {
                input_offset_y: Some(22),
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(default_anchors.auth_y, 123);
        assert_eq!(shifted_anchors.auth_y, 145);
    }

    #[test]
    fn bottom_alignment_clamps_offset_instead_of_recentering() {
        let default_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::BottomCenter,
            AnchorOffsets::default(),
        );
        let shifted_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::BottomCenter,
            AnchorOffsets {
                input_offset_y: Some(24),
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(default_anchors.auth_y, 451);
        assert_eq!(shifted_anchors.auth_y, 451);
    }

    #[test]
    fn applies_configured_input_vertical_padding() {
        let default_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::TopCenter,
            AnchorOffsets::default(),
        );
        let shifted_anchors = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::TopCenter,
            AnchorOffsets {
                input_vertical_padding: Some(180),
                ..AnchorOffsets::default()
            },
        );

        assert_eq!(default_anchors.auth_y, 123);
        assert_eq!(shifted_anchors.auth_y, 180);
    }

    #[test]
    fn supports_top_and_bottom_auth_alignment() {
        let top = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::TopCenter,
            AnchorOffsets::default(),
        );
        let bottom = role_anchors(
            720,
            54,
            197,
            197,
            FooterHeights::same(0),
            InputAlignment::BottomCenter,
            AnchorOffsets::default(),
        );

        assert!(top.auth_y < 262);
        assert!(bottom.auth_y > 262);
    }
}
