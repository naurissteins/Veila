use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{self, SeqAccess, Visitor},
    ser::SerializeSeq,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigColor(pub u8, pub u8, pub u8, pub u8);

impl ConfigColor {
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(red, green, blue, u8::MAX)
    }

    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self(red, green, blue, alpha)
    }
}

impl Serialize for ConfigColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(if self.3 == u8::MAX { 3 } else { 4 }))?;
        sequence.serialize_element(&self.0)?;
        sequence.serialize_element(&self.1)?;
        sequence.serialize_element(&self.2)?;
        if self.3 != u8::MAX {
            sequence.serialize_element(&self.3)?;
        }
        sequence.end()
    }
}

impl<'de> Deserialize<'de> for ConfigColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ConfigColorVisitor)
    }
}

struct ConfigColorVisitor;

impl<'de> Visitor<'de> for ConfigColorVisitor {
    type Value = ConfigColor;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("an RGB/RGBA array, #RRGGBB or #RRGGBBAA hex string, or rgb()/rgba() string")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let red = sequence
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let green = sequence
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let blue = sequence
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?;
        let alpha = sequence.next_element()?.unwrap_or(u8::MAX);

        if sequence.next_element::<u8>()?.is_some() {
            return Err(de::Error::invalid_length(5, &self));
        }

        Ok(ConfigColor(red, green, blue, alpha))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        parse_color(value).map_err(E::custom)
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&value)
    }
}

fn parse_color(input: &str) -> Result<ConfigColor, String> {
    let value = input.trim();

    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if let Some(body) = value
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return parse_function_color(body, false);
    }

    if let Some(body) = value
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return parse_function_color(body, true);
    }

    Err(format!("unsupported color format: {value}"))
}

fn parse_hex_color(hex: &str) -> Result<ConfigColor, String> {
    match hex.len() {
        6 => Ok(ConfigColor(
            parse_hex_byte(&hex[0..2])?,
            parse_hex_byte(&hex[2..4])?,
            parse_hex_byte(&hex[4..6])?,
            u8::MAX,
        )),
        8 => Ok(ConfigColor(
            parse_hex_byte(&hex[0..2])?,
            parse_hex_byte(&hex[2..4])?,
            parse_hex_byte(&hex[4..6])?,
            parse_hex_byte(&hex[6..8])?,
        )),
        _ => Err(format!(
            "hex colors must be #RRGGBB or #RRGGBBAA, got #{hex}"
        )),
    }
}

fn parse_hex_byte(component: &str) -> Result<u8, String> {
    u8::from_str_radix(component, 16).map_err(|_| format!("invalid hex component: {component}"))
}

fn parse_function_color(body: &str, has_alpha: bool) -> Result<ConfigColor, String> {
    let parts = body.split(',').map(str::trim).collect::<Vec<_>>();
    let expected = if has_alpha { 4 } else { 3 };
    if parts.len() != expected {
        return Err(format!(
            "expected {expected} components in {}({body})",
            if has_alpha { "rgba" } else { "rgb" }
        ));
    }

    let red = parse_channel(parts[0])?;
    let green = parse_channel(parts[1])?;
    let blue = parse_channel(parts[2])?;
    let alpha = if has_alpha {
        parse_alpha(parts[3])?
    } else {
        u8::MAX
    };

    Ok(ConfigColor(red, green, blue, alpha))
}

fn parse_channel(component: &str) -> Result<u8, String> {
    component
        .parse::<u8>()
        .map_err(|_| format!("invalid color channel: {component}"))
}

fn parse_alpha(component: &str) -> Result<u8, String> {
    if let Some(percent) = component.strip_suffix('%') {
        let percent = percent
            .trim()
            .parse::<f32>()
            .map_err(|_| format!("invalid alpha percentage: {component}"))?;
        return normalize_unit_alpha(percent / 100.0, component);
    }

    if component.contains('.') {
        let alpha = component
            .parse::<f32>()
            .map_err(|_| format!("invalid alpha value: {component}"))?;
        return normalize_unit_alpha(alpha, component);
    }

    component
        .parse::<u8>()
        .map_err(|_| format!("invalid alpha channel: {component}"))
}

fn normalize_unit_alpha(alpha: f32, original: &str) -> Result<u8, String> {
    if !(0.0..=1.0).contains(&alpha) {
        return Err(format!("alpha must be between 0.0 and 1.0: {original}"));
    }

    Ok((alpha * 255.0).round().clamp(0.0, 255.0) as u8)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::ConfigColor;

    #[derive(Debug, Deserialize)]
    struct ColorDoc {
        color: ConfigColor,
    }

    #[test]
    fn parses_rgb_array() {
        let parsed: ColorDoc = toml::from_str("color = [24, 30, 42]").expect("color");

        assert_eq!(parsed.color, ConfigColor::rgb(24, 30, 42));
    }

    #[test]
    fn parses_rgba_array() {
        let parsed: ColorDoc = toml::from_str("color = [24, 30, 42, 180]").expect("color");

        assert_eq!(parsed.color, ConfigColor::rgba(24, 30, 42, 180));
    }

    #[test]
    fn parses_hex_color() {
        let parsed: ColorDoc = toml::from_str("color = \"#181E2A\"").expect("color");

        assert_eq!(parsed.color, ConfigColor::rgb(24, 30, 42));
    }

    #[test]
    fn parses_hex_color_with_alpha() {
        let parsed: ColorDoc = toml::from_str("color = \"#181E2ACC\"").expect("color");

        assert_eq!(parsed.color, ConfigColor::rgba(24, 30, 42, 204));
    }

    #[test]
    fn parses_rgba_function_with_fractional_alpha() {
        let parsed: ColorDoc =
            toml::from_str("color = \"rgba(96, 164, 255, 0.5)\"").expect("color");

        assert_eq!(parsed.color, ConfigColor::rgba(96, 164, 255, 128));
    }
}
