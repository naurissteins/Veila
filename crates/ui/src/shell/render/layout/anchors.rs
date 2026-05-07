use veila_common::{ClockAlignment, HorizontalAlign, VerticalAlign};

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
        offsets,
    })
}

pub fn role_anchors_with_groups(input: RoleAnchorInput) -> RoleAnchors {
    let frame_height = input.frame_height;
    let hero_height = input.hero_height;
    let auth_anchor_height = input.auth_anchor_height;
    let auth_render_height = input.auth_render_height;
    let footer_heights = input.footer_heights;
    let offsets = input.offsets;
    let hero_top = top_role_top(frame_height);
    let hero_y = match offsets.clock_alignment {
        ClockAlignment::TopCenter => hero_top,
        ClockAlignment::TopRight => hero_top,
        ClockAlignment::TopLeft => hero_top,
        ClockAlignment::CenterCenter => centered_role_top(frame_height, hero_height, 0.5),
    } + offsets.clock_offset_y.unwrap_or(0);
    let footer_y = frame_height - footer_heights.render;
    let hero_bottom = hero_y + hero_height;
    let minimum_gap = if hero_height > 0 && auth_anchor_height > 0 {
        18
    } else {
        0
    };
    let centered_auth_y = centered_role_top(frame_height, auth_anchor_height, 0.5);
    let auth_footer_y = frame_height - footer_heights.clearance;
    let min_auth_y = hero_bottom + minimum_gap;
    let max_auth_y = auth_footer_y - auth_render_height - 24;

    if matches!(offsets.clock_alignment, ClockAlignment::CenterCenter)
        && hero_height > 0
        && auth_anchor_height > 0
    {
        let combined_height = hero_height + minimum_gap + auth_anchor_height;
        let group_shift = offsets.clock_offset_y.unwrap_or(0);
        let centered_hero_y = centered_role_top(frame_height, combined_height, 0.5).clamp(
            hero_top,
            (max_auth_y - hero_height - minimum_gap).max(hero_top),
        ) + group_shift;
        let auth_y = (centered_hero_y + hero_height + minimum_gap)
            .clamp(centered_hero_y + hero_height + minimum_gap, max_auth_y);

        return RoleAnchors {
            identity_y: None,
            hero_y: centered_hero_y,
            auth_y,
            footer_y,
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

    let auth_y = centered_auth_y.clamp(min_auth_y, max_auth_y);

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

pub fn anchored_block_x(
    frame_width: i32,
    block_width: i32,
    halign: HorizontalAlign,
    offset_x: i32,
) -> i32 {
    match halign {
        HorizontalAlign::Left => offset_x,
        HorizontalAlign::Center => frame_width / 2 - block_width / 2 + offset_x,
        HorizontalAlign::Right => frame_width - block_width + offset_x,
    }
}

pub fn anchored_block_y(
    frame_height: i32,
    block_height: i32,
    valign: VerticalAlign,
    offset_y: i32,
) -> i32 {
    match valign {
        VerticalAlign::Top => offset_y,
        VerticalAlign::Center => frame_height / 2 - block_height / 2 + offset_y,
        VerticalAlign::Bottom => frame_height - block_height + offset_y,
    }
}

pub fn top_role_top(frame_height: i32) -> i32 {
    (frame_height / 14).clamp(28, 72)
}

fn centered_role_top(frame_height: i32, role_height: i32, center_factor: f32) -> i32 {
    ((frame_height as f32) * center_factor) as i32 - role_height / 2
}
