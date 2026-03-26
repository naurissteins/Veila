use veila_renderer::ClearColor;

pub(super) fn clock_scale(avatar_size: i32) -> u32 {
    if avatar_size < 100 { 4 } else { 5 }
}

pub(super) fn avatar_background_color(base: ClearColor, opacity_percent: Option<u8>) -> ClearColor {
    let alpha = match opacity_percent {
        Some(percent) => percent_to_alpha(percent),
        None if base.alpha == u8::MAX => 104,
        None => base.alpha,
    };

    base.with_alpha(alpha)
}

pub(super) fn avatar_ring_color(base: ClearColor, fallback_alpha: u8) -> ClearColor {
    let alpha = if base.alpha == u8::MAX {
        fallback_alpha
    } else {
        base.alpha
    };

    base.with_alpha(alpha)
}

pub(super) fn username_color(base: ClearColor, opacity_percent: Option<u8>) -> ClearColor {
    let alpha = match opacity_percent {
        Some(percent) => percent_to_alpha(percent),
        None if base.alpha == u8::MAX => 214,
        None => base.alpha,
    };

    base.with_alpha(alpha)
}

pub(super) fn header_color(
    base: ClearColor,
    opacity_percent: Option<u8>,
    fallback_alpha: u8,
) -> ClearColor {
    let alpha = match opacity_percent {
        Some(percent) => percent_to_alpha(percent),
        None if base.alpha == u8::MAX => fallback_alpha,
        None => base.alpha,
    };

    base.with_alpha(alpha)
}

pub(super) fn secondary_text_color(
    base: ClearColor,
    opacity_percent: Option<u8>,
    fallback_alpha: u8,
) -> ClearColor {
    let alpha = match opacity_percent {
        Some(percent) => percent_to_alpha(percent),
        None if base.alpha == u8::MAX => fallback_alpha,
        None => base.alpha,
    };

    base.with_alpha(alpha)
}

pub(super) fn scaled_alpha(base_alpha: u8, opacity_percent: Option<u8>) -> u8 {
    match opacity_percent {
        Some(percent) => ((u16::from(base_alpha) * u16::from(percent.min(100)) + 50) / 100) as u8,
        None => base_alpha,
    }
}

pub(crate) fn percent_to_alpha(percent: u8) -> u8 {
    ((u16::from(percent.min(100)) * 255 + 50) / 100) as u8
}

pub(super) fn styled_alpha(configured_alpha: u8, fallback_alpha: u8) -> u8 {
    if configured_alpha == u8::MAX {
        fallback_alpha
    } else {
        configured_alpha
    }
}

pub(super) fn eye_icon_alpha(
    base_alpha: u8,
    opacity_percent: Option<u8>,
    interaction_alpha: u8,
) -> u8 {
    let effective_percent = match opacity_percent {
        Some(percent) => percent.min(100),
        None => ((u16::from(base_alpha) * 100 + 127) / 255) as u8,
    };
    ((u16::from(interaction_alpha) * u16::from(effective_percent) + 50) / 100) as u8
}
