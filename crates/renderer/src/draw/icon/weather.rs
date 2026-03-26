#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherIcon {
    ClearDay,
    ClearNight,
    PartlyCloudyDay,
    PartlyCloudyNight,
    Cloudy,
    Overcast,
    Fog,
    Drizzle,
    Rain,
    Snow,
    Thunderstorm,
    Unknown,
}

pub(super) fn weather_svg(icon: WeatherIcon) -> &'static [u8] {
    match icon {
        WeatherIcon::ClearDay => {
            include_bytes!("../../../../../assets/icons/weather/clear-day.svg")
        }
        WeatherIcon::ClearNight => {
            include_bytes!("../../../../../assets/icons/weather/clear-night.svg")
        }
        WeatherIcon::PartlyCloudyDay => {
            include_bytes!("../../../../../assets/icons/weather/partly-cloudy-day.svg")
        }
        WeatherIcon::PartlyCloudyNight => {
            include_bytes!("../../../../../assets/icons/weather/partly-cloudy-night.svg")
        }
        WeatherIcon::Cloudy => include_bytes!("../../../../../assets/icons/weather/cloudy.svg"),
        WeatherIcon::Overcast => {
            include_bytes!("../../../../../assets/icons/weather/overcast.svg")
        }
        WeatherIcon::Fog => include_bytes!("../../../../../assets/icons/weather/fog.svg"),
        WeatherIcon::Drizzle => include_bytes!("../../../../../assets/icons/weather/drizzle.svg"),
        WeatherIcon::Rain => include_bytes!("../../../../../assets/icons/weather/rain.svg"),
        WeatherIcon::Snow => include_bytes!("../../../../../assets/icons/weather/snow.svg"),
        WeatherIcon::Thunderstorm => {
            include_bytes!("../../../../../assets/icons/weather/thunderstorms.svg")
        }
        WeatherIcon::Unknown => {
            include_bytes!("../../../../../assets/icons/weather/not-available.svg")
        }
    }
}
