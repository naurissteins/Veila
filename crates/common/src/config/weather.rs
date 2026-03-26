use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GeoCoordinate(i32);

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
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            location: None,
            latitude: None,
            longitude: None,
            refresh_minutes: default_refresh_minutes(),
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
