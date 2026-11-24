use super::{
    render::layout::{anchored_block_x, anchored_block_y},
    theme::{Backdrop, scale_backdrop},
};
use std::path::Path;
use veila_common::{BackdropMode, BackdropShowWhen};
use veila_renderer::{
    ClearColor, FrameSize, PixelBuffer, RenderScale, SoftwareBuffer,
    background::{
        BackgroundTreatment, GeneratedBackground, store_cached_generated_render_variant,
        store_cached_render_variant,
    },
    layer::{BackdropLayerMode, BackdropLayerShape, BackdropLayerStyle, draw_backdrop_layer},
    shape::Rect,
};

/// immutable backdrop data for workers, contains no input, fonts or live widget state
#[derive(Debug, Clone)]
pub struct StaticBackdrops {
    backdrops: Vec<Backdrop>,
}

impl StaticBackdrops {
    pub(super) fn new(backdrops: Vec<Backdrop>) -> Self {
        Self { backdrops }
    }
    pub fn variant(&self) -> Option<String> {
        if self.backdrops.is_empty() {
            return None;
        }

        let mut variant = String::from("backdrop:v3");
        for backdrop in &self.backdrops {
            let visible = backdrop.show_when == veila_common::BackdropShowWhen::Always;
            let border = backdrop
                .border_color
                .unwrap_or(ClearColor::rgba(0, 0, 0, 0));
            variant.push_str(&format!(
                ":{:?}:{}:{}:{:?}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
                backdrop.mode,
                backdrop.show_when as u8,
                visible as u8,
                backdrop.position.halign,
                backdrop.position.valign as u8,
                backdrop.position.x,
                backdrop.position.y,
                backdrop.full_width as u8,
                backdrop.full_height as u8,
                backdrop.inset_top,
                backdrop.inset_bottom,
                backdrop.inset_left,
                backdrop.inset_right,
                backdrop.width,
                backdrop.height,
                backdrop.rotate,
                backdrop.z,
                backdrop.color.red,
                backdrop.color.green,
                backdrop.color.blue,
                backdrop.color.alpha,
                backdrop.blur_strength,
                backdrop.radius,
                backdrop.border_width,
            ));
            variant.push_str(&format!(
                ":{}:{}:{}:{}",
                border.red, border.green, border.blue, border.alpha
            ));
        }
        Some(variant)
    }
    pub fn variant_at_scale(&self, scale: RenderScale) -> Option<String> {
        let variant = self.variant()?;
        match scale.units() {
            120 => Some(variant),
            units if scale.is_integer() => Some(format!("{variant}:render-scale:{}", units / 120)),
            units => Some(format!("{variant}:render-scale-120:{units}")),
        }
    }
    pub fn render(&self, buffer: &mut impl PixelBuffer, scale: RenderScale) {
        for backdrop in &self.backdrops {
            if backdrop.show_when == BackdropShowWhen::Always {
                draw(buffer, &scale_backdrop(backdrop.clone(), scale), scale);
            }
        }
    }
    pub fn store_file(
        &self,
        path: &Path,
        fallback: ClearColor,
        treatment: BackgroundTreatment,
        scale: RenderScale,
        buffer: &mut SoftwareBuffer,
    ) -> veila_renderer::Result<()> {
        if let Some(variant) = self.variant_at_scale(scale) {
            self.render(buffer, scale);
            store_cached_render_variant(
                path,
                fallback,
                buffer.size(),
                treatment,
                buffer,
                &variant,
            )?;
        }
        Ok(())
    }
    pub fn store_generated(
        &self,
        generated: GeneratedBackground,
        treatment: BackgroundTreatment,
        scale: RenderScale,
        buffer: &mut SoftwareBuffer,
    ) -> veila_renderer::Result<()> {
        if let Some(variant) = self.variant_at_scale(scale) {
            self.render(buffer, scale);
            store_cached_generated_render_variant(
                generated,
                buffer.size(),
                treatment,
                buffer,
                &variant,
            )?;
        }
        Ok(())
    }
}

pub(super) fn draw(buffer: &mut impl PixelBuffer, backdrop: &Backdrop, scale: RenderScale) {
    let rect = rect(buffer.size(), backdrop);
    let scale_factor = scale.units() as f32 / RenderScale::ONE.units() as f32;
    let mode = match backdrop.mode {
        BackdropMode::Solid => BackdropLayerMode::Solid,
        BackdropMode::Blur => BackdropLayerMode::Blur,
    };

    draw_backdrop_layer(
        buffer,
        rect,
        BackdropLayerStyle::new(
            mode,
            BackdropLayerShape::Panel,
            backdrop.color,
            backdrop.blur_strength,
            backdrop.radius as f32 * scale_factor,
            backdrop.border_color,
            backdrop.border_width as f32 * scale_factor,
        )
        .with_rotation(backdrop.rotate),
    );
}
pub(super) fn rect(size: FrameSize, backdrop: &Backdrop) -> Rect {
    let screen_width = size.width as i32;
    let screen_height = size.height as i32;
    let x = if backdrop.full_width {
        backdrop.inset_left.min(screen_width)
    } else {
        anchored_block_x(
            screen_width,
            backdrop.width,
            backdrop.position.halign,
            backdrop.position.x,
        )
    };
    let y = if backdrop.full_height {
        backdrop.inset_top.min(screen_height)
    } else {
        anchored_block_y(
            screen_height,
            backdrop.height,
            backdrop.position.valign,
            backdrop.position.y,
        )
    };
    let width = if backdrop.full_width {
        (screen_width - backdrop.inset_left - backdrop.inset_right).max(0)
    } else {
        backdrop.width
    };
    let height = if backdrop.full_height {
        (screen_height - backdrop.inset_top - backdrop.inset_bottom).max(0)
    } else {
        backdrop.height
    };

    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::{ShellState, ShellTheme};
    use veila_common::AppConfig;

    #[test]
    fn blurred_backdrop_keeps_full_border_width_on_sides_and_corners() {
        let config = AppConfig::from_toml_str(
            r##"
            [[visuals.backdrop]]
            mode = "blur"
            color = "#00000000"
            blur_strength = 12
            width = 330
            height = 92
            halign = "left"
            valign = "top"
            radius = 20
            border_width = 1
            border_color = "#FFFFFF"
            "##,
        )
        .expect("backdrop config");
        let shell = ShellState::new(ShellTheme::from_config(&config), None, None, false);

        for (units, second_pixel) in [(120, 0), (180, 127)] {
            let scale = RenderScale::from_units(units);
            let size = scale.frame_size(FrameSize::new(340, 100));
            let mut buffer = SoftwareBuffer::solid(size, ClearColor::opaque(0, 0, 0)).unwrap();
            shell.render_static_backdrops_at_scale(&mut buffer, scale);

            let red = |x, y| buffer.pixels()[(y * size.width as usize + x) * 4 + 2];
            assert_eq!(red(50, 0), 255, "top edge at {units}/120 scale");
            assert_eq!(red(0, 50), 255, "left edge at {units}/120 scale");
            assert!(red(50, 1).abs_diff(second_pixel) <= 1);
            assert!(red(1, 50).abs_diff(second_pixel) <= 1);

            // This row crosses the circular corner near 45 degrees.
            let radius = scale.apply_i32(20) as usize;
            let y = scale.apply_i32(6) as usize;
            let coverage: u16 = (0..radius).map(|x| u16::from(red(x, y))).sum();
            let expected = f32::from(255 + u16::from(second_pixel)) * std::f32::consts::SQRT_2;
            assert!(
                (f32::from(coverage) - expected).abs() < 55.0,
                "corner coverage {coverage} at {units}/120 scale"
            );
        }
    }
}
