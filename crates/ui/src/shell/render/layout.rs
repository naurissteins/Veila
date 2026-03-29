use veila_common::{ClockAlignment, InputAlignment};
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
    pub horizontal_padding: Option<i32>,
    pub offset_x: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RoleAnchors {
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
    pub clock_alignment: ClockAlignment,
    pub weather_bottom_padding: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FooterHeights {
    pub render: i32,
    pub clearance: i32,
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
        let auth_center_x = apply_auth_offset_x(
            auth_center_x(
                width,
                input_width,
                horizontal_padding,
                input_placement.alignment,
            ),
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

pub(super) fn role_anchors(
    frame_height: i32,
    hero_height: i32,
    auth_anchor_height: i32,
    auth_render_height: i32,
    footer_heights: FooterHeights,
    input_alignment: InputAlignment,
    offsets: AnchorOffsets,
) -> RoleAnchors {
    let hero_y = top_role_top(frame_height, offsets.header_top);
    let hero_y = match offsets.clock_alignment {
        ClockAlignment::TopCenter => hero_y,
        ClockAlignment::CenterCenter => centered_role_top(frame_height, hero_height, 0.5),
    };
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

    if max_auth_y < min_auth_y {
        let combined_height = hero_height + minimum_gap + auth_render_height;
        let combined_top = ((frame_height - combined_height) / 2)
            .max(top_role_top(frame_height, offsets.header_top));

        return RoleAnchors {
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

pub(super) fn top_role_top(frame_height: i32, header_top_offset: Option<i32>) -> i32 {
    ((frame_height / 14).clamp(28, 72) + header_top_offset.unwrap_or(0)).max(0)
}

#[cfg(test)]
mod tests {
    use veila_common::{ClockAlignment, InputAlignment};

    use super::{AnchorOffsets, FooterHeights, InputPlacement, SceneMetrics, role_anchors};

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
        assert_eq!(centered_anchors.hero_y, 333);
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
                horizontal_padding: Some(96),
                offset_x: None,
            },
        );

        assert_eq!(default_metrics.auth_center_x, 203);
        assert_eq!(shifted_metrics.auth_center_x, 246);
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
