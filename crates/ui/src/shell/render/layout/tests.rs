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
fn applies_configured_identity_gap_for_centered_grouped_layouts() {
    let default_anchors = role_anchors_with_groups(RoleAnchorInput {
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
    let widened_gap_anchors = role_anchors_with_groups(RoleAnchorInput {
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
            identity_gap: Some(30),
            ..AnchorOffsets::default()
        },
    });

    assert_eq!(default_anchors.identity_y, Some(254));
    assert_eq!(default_anchors.hero_y, 344);
    assert_eq!(widened_gap_anchors.identity_y, Some(248));
    assert_eq!(widened_gap_anchors.hero_y, 350);
    assert_eq!(widened_gap_anchors.auth_y, 422);
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
