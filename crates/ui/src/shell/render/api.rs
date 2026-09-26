use crate::shell::ShellState;
use veila_renderer::{FrameSize, PixelBuffer, shape::Rect};

impl ShellState {
    pub fn render_backdrops(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_backdrops(buffer);
    }

    pub fn render_static_backdrops(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_static_backdrops(buffer);
    }

    pub fn render_dynamic_backdrops(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_dynamic_backdrops(buffer);
    }

    pub fn render_backdrops_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render_backdrops(buffer));
    }

    pub fn render_static_backdrops_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render_static_backdrops(buffer));
    }

    pub fn render_dynamic_backdrops_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render_dynamic_backdrops(buffer));
    }

    pub fn render_layers(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_layers(buffer);
    }

    pub fn render_layers_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render_layers(buffer));
    }

    pub fn render(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render(buffer);
    }

    pub fn render_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render(buffer));
    }

    pub fn render_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_overlay(buffer);
    }

    pub fn render_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render_overlay(buffer));
    }

    pub fn render_static_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_static_overlay(buffer);
    }

    pub fn render_static_overlay_without_layers(&self, buffer: &mut impl PixelBuffer) {
        self.render_context()
            .render_static_overlay_without_layers(buffer);
    }

    pub fn render_static_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render_static_overlay(buffer));
    }

    pub fn render_static_overlay_without_layers_scaled(
        &self,
        buffer: &mut impl PixelBuffer,
        scale: u32,
    ) {
        self.with_render_scale(scale, |context| {
            context.render_static_overlay_without_layers(buffer)
        });
    }

    pub fn render_dynamic_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_dynamic_overlay(buffer);
    }

    pub fn render_dynamic_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render_dynamic_overlay(buffer));
    }

    pub fn render_auth_dirty_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.with_render_scale(scale, |context| context.render_auth_dirty_overlay(buffer));
    }

    pub fn auth_dirty_rect_scaled(&self, size: FrameSize, scale: u32) -> Option<Rect> {
        if self.emergency_active() {
            return None;
        }
        self.with_render_scale(scale, |context| context.auth_dirty_rect(size))
    }

    pub fn render_emergency(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_emergency(buffer);
    }

    pub fn render_emergency_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.emergency_render_context(scale)
            .render_emergency(buffer);
    }

    pub fn render_emergency_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_context().render_emergency_overlay(buffer);
    }

    pub fn render_emergency_overlay_scaled(&self, buffer: &mut impl PixelBuffer, scale: u32) {
        self.emergency_render_context(scale)
            .render_emergency_overlay(buffer);
    }

    pub fn render_emergency_static_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_context()
            .render_emergency_static_overlay(buffer);
    }

    pub fn render_emergency_static_overlay_scaled(
        &self,
        buffer: &mut impl PixelBuffer,
        scale: u32,
    ) {
        self.emergency_render_context(scale)
            .render_emergency_static_overlay(buffer);
    }

    pub fn render_emergency_dynamic_overlay(&self, buffer: &mut impl PixelBuffer) {
        self.render_context()
            .render_emergency_dynamic_overlay(buffer);
    }

    pub fn render_emergency_dynamic_overlay_scaled(
        &self,
        buffer: &mut impl PixelBuffer,
        scale: u32,
    ) {
        self.emergency_render_context(scale)
            .render_emergency_dynamic_overlay(buffer);
    }
}
