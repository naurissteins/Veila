use veila_common::{CenterStackStyle, ClockAlignment, InputAlignment};

use super::types::{RoleAnchorInput, RoleAnchors};

#[cfg(test)]
use super::types::{AnchorOffsets, AuthGroupHeights, FooterHeights};

#[cfg(test)]
pub fn role_anchors(
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

pub fn role_anchors_with_groups(input: RoleAnchorInput) -> RoleAnchors {
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
                    offsets.identity_gap.unwrap_or(18).clamp(0, 160)
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

pub fn hero_block_x(
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
            ClockAlignment::TopLeft => super::horizontal_auth_padding(frame_width),
            ClockAlignment::TopRight => {
                (frame_width - super::horizontal_auth_padding(frame_width) - block_width).max(0)
            }
        },
    };

    (base_x + offset_x.unwrap_or(0)).clamp(0, (frame_width - block_width).max(0))
}

pub fn top_role_top(frame_height: i32, header_top_offset: Option<i32>) -> i32 {
    ((frame_height / 14).clamp(28, 72) + header_top_offset.unwrap_or(0)).max(0)
}

fn centered_role_top(frame_height: i32, role_height: i32, center_factor: f32) -> i32 {
    ((frame_height as f32) * center_factor) as i32 - role_height / 2
}
