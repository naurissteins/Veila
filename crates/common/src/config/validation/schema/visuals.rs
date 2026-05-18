use super::{KeyRule, Schema, key};

pub(super) const VISUALS: &[KeyRule] = &[
    key("panel", Schema::Any),
    key("avatar_background_color", Schema::Any),
    key("panel_border", Schema::Any),
    key("input", Schema::Table(INPUT)),
    key("input_font_family", Schema::Any),
    key("input_font_weight", Schema::Any),
    key("input_font_style", Schema::Any),
    key("input_font_size", Schema::Any),
    key("input_border", Schema::Any),
    key("input_width", Schema::Any),
    key("input_height", Schema::Any),
    key("input_radius", Schema::Any),
    key("input_border_width", Schema::Any),
    key("avatar_size", Schema::Any),
    key("avatar_radius", Schema::Any),
    key("avatar_placeholder_padding", Schema::Any),
    key("avatar_icon_color", Schema::Any),
    key("avatar_ring_color", Schema::Any),
    key("avatar_ring_width", Schema::Any),
    key("username_color", Schema::Any),
    key("username_font_size", Schema::Any),
    key("clock_font_family", Schema::Any),
    key("clock_font_weight", Schema::Any),
    key("clock_font_style", Schema::Any),
    key("clock_style", Schema::Any),
    key("clock_format", Schema::Any),
    key("clock_meridiem_font_size", Schema::Any),
    key("clock_meridiem_x", Schema::Any),
    key("clock_meridiem_y", Schema::Any),
    key("clock_color", Schema::Any),
    key("date_color", Schema::Any),
    key("clock_font_size", Schema::Any),
    key("date_font_size", Schema::Any),
    key("placeholder_color", Schema::Any),
    key("eye_icon_color", Schema::Any),
    key("keyboard_color", Schema::Any),
    key("battery_color", Schema::Any),
    key("battery_background_color", Schema::Any),
    key("keyboard_background_size", Schema::Any),
    key("battery_background_size", Schema::Any),
    key("keyboard_size", Schema::Any),
    key("battery_size", Schema::Any),
    key("status_color", Schema::Any),
    key("input_mask_color", Schema::Any),
    key("foreground", Schema::Any),
    key("muted", Schema::Any),
    key("pending", Schema::Any),
    key("rejected", Schema::Any),
    key("avatar", Schema::Table(AVATAR)),
    key("username", Schema::Table(USERNAME)),
    key("clock", Schema::Table(CLOCK)),
    key("date", Schema::Table(DATE)),
    key("placeholder", Schema::Table(PLACEHOLDER)),
    key("reveal", Schema::Table(REVEAL)),
    key("status", Schema::Table(STATUS)),
    key("eye", Schema::Table(EYE)),
    key("caps_lock", Schema::Table(CAPS_LOCK)),
    key("keyboard", Schema::Table(ICON_CHIP)),
    key("battery", Schema::Table(ICON_CHIP)),
    key("power_status", Schema::Table(POWER_STATUS)),
    key("power", Schema::Table(POWER)),
    key("grid", Schema::Table(GRID)),
    key("weather", Schema::Table(WEATHER_VISUAL)),
    key("backdrop", Schema::ArrayTable(BACKDROP)),
    key("layer", Schema::ArrayTable(LAYER)),
    key("now_playing", Schema::Table(NOW_PLAYING_VISUAL)),
    key("outputs", Schema::Table(OUTPUTS)),
    key("palette", Schema::Table(PALETTE)),
];

const INPUT: &[KeyRule] = &[
    key("placeholder", Schema::Any),
    key("reveal_on_interaction", Schema::Any),
    key("reveal_mode", Schema::Any),
    key("reveal_hint", Schema::Any),
    key("font_family", Schema::Any),
    key("font_weight", Schema::Any),
    key("font_style", Schema::Any),
    key("font_size", Schema::Any),
    key("background_color", Schema::Any),
    key("border_color", Schema::Any),
    key("width", Schema::Any),
    key("height", Schema::Any),
    key("radius", Schema::Any),
    key("border_width", Schema::Any),
    key("mask_color", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const AVATAR: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("image_path", Schema::Any),
    key("size", Schema::Any),
    key("radius", Schema::Any),
    key("background_color", Schema::Any),
    key("placeholder_padding", Schema::Any),
    key("ring_color", Schema::Any),
    key("ring_width", Schema::Any),
    key("icon_color", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const USERNAME: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("text", Schema::Any),
    key("font_family", Schema::Any),
    key("font_weight", Schema::Any),
    key("font_style", Schema::Any),
    key("color", Schema::Any),
    key("font_size", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const CLOCK: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("font_family", Schema::Any),
    key("font_weight", Schema::Any),
    key("font_style", Schema::Any),
    key("style", Schema::Any),
    key("format", Schema::Any),
    key("meridiem_font_size", Schema::Any),
    key("meridiem_x", Schema::Any),
    key("meridiem_y", Schema::Any),
    key("color", Schema::Any),
    key("font_size", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const DATE: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("format", Schema::Any),
    key("font_family", Schema::Any),
    key("font_weight", Schema::Any),
    key("font_style", Schema::Any),
    key("color", Schema::Any),
    key("font_size", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const PLACEHOLDER: &[KeyRule] = &[key("enabled", Schema::Any), key("color", Schema::Any)];

const REVEAL: &[KeyRule] = &[
    key("mode", Schema::Any),
    key("text", Schema::Any),
    key("color", Schema::Any),
    key("font_family", Schema::Any),
    key("font_weight", Schema::Any),
    key("font_style", Schema::Any),
    key("font_size", Schema::Any),
];

const STATUS: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("mode", Schema::Any),
    key("color", Schema::Any),
    key("pending_color", Schema::Any),
    key("rejected_color", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const EYE: &[KeyRule] = &[key("enabled", Schema::Any), key("color", Schema::Any)];

const CAPS_LOCK: &[KeyRule] = &[key("enabled", Schema::Any), key("color", Schema::Any)];

const ICON_CHIP: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("background_color", Schema::Any),
    key("background_size", Schema::Any),
    key("radius", Schema::Any),
    key("color", Schema::Any),
    key("size", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const POWER_STATUS: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const POWER: &[KeyRule] = &[
    key("suspend", Schema::Table(POWER_BUTTON)),
    key("reboot", Schema::Table(POWER_BUTTON)),
    key("poweroff", Schema::Table(POWER_BUTTON)),
];

const POWER_BUTTON: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("background_color", Schema::Any),
    key("background_size", Schema::Any),
    key("radius", Schema::Any),
    key("color", Schema::Any),
    key("size", Schema::Any),
    key("confirm", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const GRID: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("cell_size", Schema::Any),
    key("color", Schema::Any),
    key("major_every", Schema::Any),
    key("major_color", Schema::Any),
];

const WEATHER_VISUAL: &[KeyRule] = &[
    key("icon", Schema::Table(WEATHER_ICON)),
    key("temperature", Schema::Table(WEATHER_TEXT)),
    key("location", Schema::Table(WEATHER_TEXT)),
];

const WEATHER_ICON: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("size", Schema::Any),
    key("opacity", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const WEATHER_TEXT: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("font_size", Schema::Any),
    key("font_family", Schema::Any),
    key("font_weight", Schema::Any),
    key("font_style", Schema::Any),
    key("letter_spacing", Schema::Any),
    key("color", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const BACKDROP: &[KeyRule] = &[
    key("name", Schema::Any),
    key("enabled", Schema::Any),
    key("show_when", Schema::Any),
    key("mode", Schema::Any),
    key("color", Schema::Any),
    key("blur_strength", Schema::Any),
    key("radius", Schema::Any),
    key("border_color", Schema::Any),
    key("border_width", Schema::Any),
    key("full_width", Schema::Any),
    key("full_height", Schema::Any),
    key("inset_top", Schema::Any),
    key("inset_bottom", Schema::Any),
    key("inset_left", Schema::Any),
    key("inset_right", Schema::Any),
    key("width", Schema::Any),
    key("height", Schema::Any),
    key("rotate", Schema::Any),
    key("z", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const LAYER: &[KeyRule] = &[
    key("name", Schema::Any),
    key("enabled", Schema::Any),
    key("kind", Schema::Any),
    key("text", Schema::Any),
    key("font_family", Schema::Any),
    key("font_weight", Schema::Any),
    key("font_style", Schema::Any),
    key("font_size", Schema::Any),
    key("color", Schema::Any),
    key("background_color", Schema::Any),
    key("width", Schema::Any),
    key("height", Schema::Any),
    key("padding", Schema::Any),
    key("radius", Schema::Any),
    key("z", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const NOW_PLAYING_VISUAL: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("fade_duration_ms", Schema::Any),
    key("artwork", Schema::Table(NOW_PLAYING_ARTWORK)),
    key("artist", Schema::Table(NOW_PLAYING_TEXT)),
    key("title", Schema::Table(NOW_PLAYING_TEXT)),
];

const NOW_PLAYING_ARTWORK: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("size", Schema::Any),
    key("radius", Schema::Any),
    key("opacity", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const NOW_PLAYING_TEXT: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("width", Schema::Any),
    key("color", Schema::Any),
    key("font_family", Schema::Any),
    key("font_size", Schema::Any),
    key("font_weight", Schema::Any),
    key("font_style", Schema::Any),
    key("halign", Schema::Any),
    key("valign", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("relative_to", Schema::Any),
];

const OUTPUTS: &[KeyRule] = &[key("ui_mode", Schema::Any), key("ui_output", Schema::Any)];

const PALETTE: &[KeyRule] = &[
    key("foreground", Schema::Any),
    key("muted", Schema::Any),
    key("pending", Schema::Any),
    key("rejected", Schema::Any),
];
