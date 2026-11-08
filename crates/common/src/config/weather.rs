use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeoCoordinate(i32);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WeatherUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WeatherConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub latitude: Option<GeoCoordinate>,
    #[serde(default)]
    pub longitude: Option<GeoCoordinate>,
    #[serde(default = "default_refresh_minutes")]
    pub refresh_minutes: u16,
    #[serde(default)]
    pub unit: WeatherUnit,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            location: None,
            latitude: None,
            longitude: None,
            refresh_minutes: default_refresh_minutes(),
            unit: WeatherUnit::default(),
        }
    }
}

impl WeatherConfig {
    pub fn coordinates(self) -> Option<(f64, f64)> {
        Some((self.latitude?.as_f64(), self.longitude?.as_f64()))
    }

    pub fn normalized_location(&self) -> Option<String> {
        self.location
            .as_deref()
            .map(str::trim)
            .filter(|location| !location.is_empty())
            .map(str::to_owned)
    }
}

impl GeoCoordinate {
    pub fn as_f64(self) -> f64 {
        f64::from(self.0) / 1_000_000.0
    }
}

impl Serialize for GeoCoordinate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Config handoff uses the same decimal-degree representation as TOML input.
        serializer.serialize_f64(self.as_f64())
    }
}

impl<'de> Deserialize<'de> for GeoCoordinate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        if !value.is_finite() {
            return Err(serde::de::Error::custom("coordinate must be finite"));
        }

        let scaled = (value * 1_000_000.0).round();
        if scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
            return Err(serde::de::Error::custom("coordinate is out of range"));
        }

        Ok(Self(scaled as i32))
    }
}

const fn default_refresh_minutes() -> u16 {
    15
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinate_json_round_trip_preserves_decimal_degrees_and_internal_precision() {
        for value in [
            -2147.483648,
            -179.999999,
            -60.123456,
            0.0,
            25.123456,
            2147.483647,
        ] {
            let coordinate: GeoCoordinate = serde_json::from_str(&value.to_string()).unwrap();
            let bytes = serde_json::to_vec(&coordinate).unwrap();
            assert_eq!(
                serde_json::from_slice::<GeoCoordinate>(&bytes).unwrap(),
                coordinate
            );
            assert_eq!(
                serde_json::from_slice::<f64>(&bytes).unwrap(),
                coordinate.as_f64()
            );
        }
    }
    #[test]
    fn resolved_config_frame_round_trips_nonzero_weather_coordinates() {
        let config = crate::AppConfig::from_toml_str(
            "[weather]\nenabled=true\nlatitude=60.123456\nlongitude=-25.654321\n",
        )
        .unwrap();
        let frame = crate::session::frame::encode(&config).unwrap();
        let restored: crate::AppConfig =
            crate::session::frame::decode(std::str::from_utf8(&frame[..frame.len() - 1]).unwrap())
                .unwrap();
        assert_eq!(restored, config);
    }
}
