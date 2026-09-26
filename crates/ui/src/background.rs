use veila_common::config::{
    BackgroundConfig, BackgroundLayeredBaseMode, BackgroundLayeredConfig,
    BackgroundScaling as ConfigBackgroundScaling,
};
use veila_renderer::{
    ClearColor,
    background::{
        BackgroundGradient, BackgroundLayered, BackgroundLayeredBase, BackgroundLayeredBlob,
        BackgroundRadial, BackgroundScaling, BackgroundTreatment, GeneratedBackground,
    },
};

pub fn background_treatment(config: &BackgroundConfig) -> BackgroundTreatment {
    BackgroundTreatment {
        blur_radius: config.blur_strength,
        dim_strength: config.dim_strength,
        tint: config
            .tint
            .map(|color| ClearColor::rgba(color.0, color.1, color.2, color.3)),
        scaling: to_background_scaling(config.scaling),
    }
}

fn to_background_scaling(scaling: ConfigBackgroundScaling) -> BackgroundScaling {
    match scaling {
        ConfigBackgroundScaling::Fill => BackgroundScaling::Fill,
        ConfigBackgroundScaling::Fit => BackgroundScaling::Fit,
        ConfigBackgroundScaling::Center => BackgroundScaling::Center,
        ConfigBackgroundScaling::Tile => BackgroundScaling::Tile,
        ConfigBackgroundScaling::Stretch => BackgroundScaling::Stretch,
    }
}

pub fn background_generated(config: &BackgroundConfig) -> Option<GeneratedBackground> {
    if let Some(gradient) = config.resolved_gradient() {
        return Some(GeneratedBackground::Gradient(BackgroundGradient {
            top_left: to_background_color(gradient.top_left),
            top_right: to_background_color(gradient.top_right),
            bottom_left: to_background_color(gradient.bottom_left),
            bottom_right: to_background_color(gradient.bottom_right),
        }));
    }

    if let Some(radial) = config.resolved_radial() {
        return Some(GeneratedBackground::Radial(BackgroundRadial {
            center: to_background_color(radial.center),
            edge: to_background_color(radial.edge),
            center_x: radial.center_x,
            center_y: radial.center_y,
            radius: radial.radius,
        }));
    }

    config
        .resolved_layered()
        .map(|layered| GeneratedBackground::Layered(to_layered_background(&layered)))
}

fn to_background_color(color: veila_common::RgbColor) -> ClearColor {
    ClearColor::rgba(color.0, color.1, color.2, color.3)
}

fn to_layered_background(config: &BackgroundLayeredConfig) -> BackgroundLayered {
    let base = match config.base.effective_mode() {
        BackgroundLayeredBaseMode::Gradient => {
            let gradient = config.base.gradient.clone().unwrap_or_default();
            BackgroundLayeredBase::Gradient(BackgroundGradient {
                top_left: to_background_color(gradient.top_left),
                top_right: to_background_color(gradient.top_right),
                bottom_left: to_background_color(gradient.bottom_left),
                bottom_right: to_background_color(gradient.bottom_right),
            })
        }
        BackgroundLayeredBaseMode::Radial => {
            let radial = config.base.radial.clone().unwrap_or_default();
            BackgroundLayeredBase::Radial(BackgroundRadial {
                center: to_background_color(radial.center),
                edge: to_background_color(radial.edge),
                center_x: radial.center_x,
                center_y: radial.center_y,
                radius: radial.radius,
            })
        }
        BackgroundLayeredBaseMode::Solid => {
            BackgroundLayeredBase::Solid(to_background_color(config.base.color))
        }
    };

    let mut blobs = [None; 3];
    for (slot, blob) in blobs.iter_mut().zip(config.blobs.iter().take(3)) {
        *slot = Some(BackgroundLayeredBlob {
            color: blob_color(blob.color, blob.opacity),
            x: blob.x,
            y: blob.y,
            size: blob.size,
        });
    }

    BackgroundLayered { base, blobs }
}

fn blob_color(color: veila_common::RgbColor, opacity: u8) -> ClearColor {
    let alpha = ((u16::from(color.3) * u16::from(opacity.min(100)) + 50) / 100) as u8;
    ClearColor::rgba(color.0, color.1, color.2, alpha)
}
