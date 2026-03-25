use veila_renderer::shape::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SceneMetrics {
    pub center_x: i32,
    pub content_width: u32,
    pub clock_width: u32,
    pub input_width: i32,
    pub input_height: i32,
    pub avatar_size: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RoleAnchors {
    pub hero_y: i32,
    pub auth_y: i32,
    pub footer_y: i32,
}

impl SceneMetrics {
    pub(super) fn from_frame(width: i32, height: i32) -> Self {
        let scene_center_x = width / 2;
        let scene_width = ((width as f32) * 0.34) as i32;
        let input_width = (((scene_width as f32) * 0.7) as i32).clamp(220, 320);
        let input_height = (((height as f32) * 0.072) as i32).clamp(48, 58);
        let avatar_size = (width.min(height) / 7).clamp(84, 108);

        Self {
            center_x: scene_center_x,
            content_width: (input_width + 72).max(220) as u32,
            clock_width: (input_width + 140).max(280) as u32,
            input_width,
            input_height,
            avatar_size,
        }
    }

    pub(super) fn input_rect(self, y: i32) -> Rect {
        Rect::new(
            self.center_x - self.input_width / 2,
            y,
            self.input_width,
            self.input_height,
        )
    }
}

pub(super) fn role_anchors(
    frame_height: i32,
    hero_height: i32,
    auth_height: i32,
    footer_height: i32,
) -> RoleAnchors {
    let hero_y = top_role_top(frame_height);
    let footer_y = frame_height - footer_height - 48;
    let hero_bottom = hero_y + hero_height;
    let minimum_gap = if hero_height > 0 && auth_height > 0 {
        18
    } else {
        0
    };
    let mut auth_y = centered_role_top(frame_height, auth_height, 0.5);

    if auth_y < hero_bottom + minimum_gap {
        auth_y = hero_bottom + minimum_gap;
    }

    if auth_y + auth_height > footer_y - 24 {
        let combined_height = hero_height + minimum_gap + auth_height;
        let combined_top = ((frame_height - combined_height) / 2).max(top_role_top(frame_height));

        return RoleAnchors {
            hero_y: combined_top,
            auth_y: combined_top + hero_height + minimum_gap,
            footer_y,
        };
    }

    RoleAnchors {
        hero_y,
        auth_y,
        footer_y,
    }
}

fn centered_role_top(frame_height: i32, role_height: i32, center_factor: f32) -> i32 {
    ((frame_height as f32) * center_factor) as i32 - role_height / 2
}

fn top_role_top(frame_height: i32) -> i32 {
    (frame_height / 14).clamp(28, 72)
}

#[cfg(test)]
mod tests {
    use super::{SceneMetrics, role_anchors};

    #[test]
    fn falls_back_to_stacked_roles_when_they_would_overlap() {
        let anchors = role_anchors(400, 160, 170, 0);

        assert_eq!(anchors.hero_y, 28);
        assert_eq!(anchors.auth_y, 206);
    }

    #[test]
    fn uses_slimmer_input_height() {
        let metrics = SceneMetrics::from_frame(1280, 720);

        assert_eq!(metrics.input_height, 51);
    }

    #[test]
    fn uses_narrower_input_width() {
        let metrics = SceneMetrics::from_frame(1280, 720);

        assert_eq!(metrics.input_width, 304);
    }

    #[test]
    fn uses_smaller_avatar_size_for_compact_hero_stack() {
        let metrics = SceneMetrics::from_frame(1280, 720);

        assert_eq!(metrics.avatar_size, 102);
    }

    #[test]
    fn keeps_auth_close_to_hero_when_space_allows() {
        let anchors = role_anchors(720, 54, 197, 0);

        assert_eq!(anchors.hero_y, 51);
        assert_eq!(anchors.auth_y, 262);
    }
}
