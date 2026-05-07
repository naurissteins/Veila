use veila_common::{
    ClockAlignment, HorizontalAlign, LayerAlignment, LayerVerticalAlignment, VerticalAlign,
};

use super::{
    AnchorOffsets, FooterHeights, LayerPlacement, SceneMetrics, anchored_block_x, anchored_block_y,
    hero_block_x, layer_center_x, layer_rect, role_anchors,
};

#[test]
fn falls_back_to_stacked_roles_when_they_would_overlap() {
    let anchors = role_anchors(
        400,
        160,
        170,
        170,
        FooterHeights::same(0),
        AnchorOffsets::default(),
    );

    assert_eq!(anchors.hero_y, 28);
    assert_eq!(anchors.auth_y, 206);
}

#[test]
fn uses_slimmer_input_height() {
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    assert_eq!(metrics.input_height, 51);
}

#[test]
fn uses_narrower_input_width() {
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    assert_eq!(metrics.input_width, 304);
}

#[test]
fn uses_smaller_avatar_size_for_compact_hero_stack() {
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    assert_eq!(metrics.avatar_size, 102);
}

#[test]
fn uses_configured_avatar_size_when_present() {
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, Some(88));

    assert_eq!(metrics.avatar_size, 88);
}

#[test]
fn uses_configured_input_width_when_present() {
    let metrics = SceneMetrics::from_frame(1280, 720, Some(280), None, None);

    assert_eq!(metrics.input_width, 280);
}

#[test]
fn uses_configured_input_height_when_present() {
    let metrics = SceneMetrics::from_frame(1280, 720, None, Some(54), None);

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
        AnchorOffsets::default(),
    );
    let with_status = role_anchors(
        720,
        54,
        197,
        235,
        FooterHeights::same(0),
        AnchorOffsets::default(),
    );

    assert_eq!(without_status.auth_y, 262);
    assert_eq!(with_status.auth_y, 262);
}

#[test]
fn supports_centered_clock_alignment() {
    let default_anchors = role_anchors(
        720,
        54,
        197,
        197,
        FooterHeights::same(0),
        AnchorOffsets::default(),
    );
    let centered_anchors = role_anchors(
        720,
        54,
        197,
        197,
        FooterHeights::same(0),
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
fn anchors_explicit_widget_positions() {
    assert_eq!(anchored_block_x(1280, 300, HorizontalAlign::Left, 20), 20);
    assert_eq!(
        anchored_block_x(1280, 300, HorizontalAlign::Center, 24),
        514
    );
    assert_eq!(
        anchored_block_x(1280, 300, HorizontalAlign::Right, -18),
        962
    );
    assert_eq!(anchored_block_y(720, 120, VerticalAlign::Top, 32), 32);
    assert_eq!(anchored_block_y(720, 120, VerticalAlign::Center, -10), 290);
    assert_eq!(anchored_block_y(720, 120, VerticalAlign::Bottom, -40), 560);
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
        AnchorOffsets {
            clock_offset_y: Some(18),
            ..AnchorOffsets::default()
        },
    );

    assert_eq!(anchors.hero_y, 69);
}

#[test]
fn applies_configured_weather_bottom_padding() {
    let default_anchors = role_anchors(
        720,
        54,
        197,
        197,
        FooterHeights::same(80),
        AnchorOffsets::default(),
    );
    let shifted_anchors = role_anchors(
        720,
        54,
        197,
        197,
        FooterHeights::same(80),
        AnchorOffsets {
            weather_bottom_padding: Some(72),
            ..AnchorOffsets::default()
        },
    );

    assert_eq!(default_anchors.footer_y, 592);
    assert_eq!(shifted_anchors.footer_y, 568);
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
