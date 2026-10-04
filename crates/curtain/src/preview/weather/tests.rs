use super::*;

#[test]
fn uses_day_icon_during_daylight_preview_hours() {
    assert_eq!(
        preview_weather_condition_for_hour(6),
        WeatherCondition::ClearDay
    );
    assert_eq!(
        preview_weather_condition_for_hour(12),
        WeatherCondition::ClearDay
    );
    assert_eq!(
        preview_weather_condition_for_hour(17),
        WeatherCondition::ClearDay
    );
}

#[test]
fn uses_night_icon_outside_daylight_preview_hours() {
    assert_eq!(
        preview_weather_condition_for_hour(0),
        WeatherCondition::ClearNight
    );
    assert_eq!(
        preview_weather_condition_for_hour(5),
        WeatherCondition::ClearNight
    );
    assert_eq!(
        preview_weather_condition_for_hour(18),
        WeatherCondition::ClearNight
    );
    assert_eq!(
        preview_weather_condition_for_hour(23),
        WeatherCondition::ClearNight
    );
}

#[test]
fn preview_weather_override_uses_requested_condition_and_temperature() {
    let options = CurtainOptions {
        preview_weather_condition: Some(WeatherCondition::Snow),
        preview_weather_temperature_celsius: Some(-4),
        ..CurtainOptions::default()
    };

    assert_eq!(
        preview_weather_override_snapshot(&options),
        Some(WeatherSnapshot {
            temperature_celsius: -4,
            condition: WeatherCondition::Snow,
            fetched_at_unix: 0,
        })
    );
}

#[test]
fn preview_weather_location_prefers_override() {
    let options = CurtainOptions {
        preview_weather_location: Some(String::from("Tokyo")),
        ..CurtainOptions::default()
    };
    let config = AppConfig::from_toml_str(
        r#"
            [weather]
            enabled = true
            location = "Riga"
        "#,
    )
    .expect("config");

    assert_eq!(
        preview_weather_location(&options, &config),
        Some(String::from("Tokyo"))
    );
}

#[test]
fn preview_weather_respects_disabled_weather_fetch_without_override() {
    let options = CurtainOptions::default();
    let config = AppConfig::from_toml_str(
        r#"
            [weather]
            enabled = false
            location = "Riga"
        "#,
    )
    .expect("config");

    assert!(preview_weather_hidden(&options, &config));
    assert!(!preview_weather_forced(&options));
}

#[test]
fn preview_weather_respects_disabled_weather_parts_without_override() {
    let options = CurtainOptions::default();
    let config = AppConfig::from_toml_str(
        r#"
            [weather]
            enabled = true
            location = "Riga"

            [visuals.weather.icon]
            enabled = false

            [visuals.weather.temperature]
            enabled = false

            [visuals.weather.location]
            enabled = false
        "#,
    )
    .expect("config");

    assert!(preview_weather_hidden(&options, &config));
    assert!(!preview_weather_forced(&options));
}

#[test]
fn preview_weather_override_can_force_preview_when_weather_is_disabled() {
    let options = CurtainOptions {
        preview_weather_location: Some(String::from("Tokyo")),
        ..CurtainOptions::default()
    };
    let config = AppConfig::from_toml_str(
        r#"
            [weather]
            enabled = false
        "#,
    )
    .expect("config");

    assert!(preview_weather_forced(&options));
    assert!(!preview_weather_hidden(&options, &config));
}
