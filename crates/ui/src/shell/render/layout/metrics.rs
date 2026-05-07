use veila_renderer::shape::Rect;

use super::SceneMetrics;

impl SceneMetrics {
    #[cfg(test)]
    pub fn from_frame(
        width: i32,
        height: i32,
        configured_input_width: Option<i32>,
        configured_input_height: Option<i32>,
        configured_avatar_size: Option<i32>,
    ) -> Self {
        Self::new(
            width,
            height,
            configured_input_width,
            configured_input_height,
            configured_avatar_size,
        )
    }

    pub fn new(
        width: i32,
        height: i32,
        configured_input_width: Option<i32>,
        configured_input_height: Option<i32>,
        configured_avatar_size: Option<i32>,
    ) -> Self {
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
        Self {
            center_x: width / 2,
            auth_center_x: width / 2,
            content_width: (input_width + 72).max(220) as u32,
            clock_width: (input_width + 140).max(280) as u32,
            input_width,
            input_height,
            avatar_size,
        }
    }

    pub fn input_rect(self, y: i32) -> Rect {
        Rect::new(
            self.auth_center_x - self.input_width / 2,
            y,
            self.input_width,
            self.input_height,
        )
    }
}
