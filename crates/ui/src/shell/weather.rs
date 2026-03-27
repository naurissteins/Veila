use veila_common::{WeatherCondition, WeatherSnapshot, WeatherUnit};
use veila_renderer::icon::WeatherIcon;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WeatherWidgetData {
    pub(super) location: String,
    pub(super) temperature_text: String,
    pub(super) icon: WeatherIcon,
}

pub(super) fn widget_data(
    location: Option<String>,
    snapshot: Option<WeatherSnapshot>,
    unit: WeatherUnit,
) -> Option<WeatherWidgetData> {
    let location = location
        .as_deref()
        .map(str::trim)
        .filter(|location| !location.is_empty())
        .map(str::to_owned)?;
    let snapshot = snapshot?;

    Some(WeatherWidgetData {
        location,
        temperature_text: format_temperature(snapshot.temperature_celsius, unit),
        icon: icon_for_condition(snapshot.condition),
    })
}

fn format_temperature(temperature_celsius: i16, unit: WeatherUnit) -> String {
    match unit {
        WeatherUnit::Celsius => format!("{temperature_celsius}°C"),
        WeatherUnit::Fahrenheit => {
            let temperature_fahrenheit =
                ((f32::from(temperature_celsius) * 9.0 / 5.0) + 32.0).round() as i16;
            format!("{temperature_fahrenheit}°F")
        }
    }
}

fn icon_for_condition(condition: WeatherCondition) -> WeatherIcon {
    match condition {
        WeatherCondition::ClearDay => WeatherIcon::ClearDay,
        WeatherCondition::ClearNight => WeatherIcon::ClearNight,
        WeatherCondition::PartlyCloudyDay => WeatherIcon::PartlyCloudyDay,
        WeatherCondition::PartlyCloudyNight => WeatherIcon::PartlyCloudyNight,
        WeatherCondition::Cloudy => WeatherIcon::Cloudy,
        WeatherCondition::Overcast => WeatherIcon::Overcast,
        WeatherCondition::Fog => WeatherIcon::Fog,
        WeatherCondition::Drizzle => WeatherIcon::Drizzle,
        WeatherCondition::Rain => WeatherIcon::Rain,
        WeatherCondition::Snow => WeatherIcon::Snow,
        WeatherCondition::Thunderstorm => WeatherIcon::Thunderstorm,
        WeatherCondition::Unknown => WeatherIcon::Unknown,
    }
}
