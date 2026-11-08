use veila_common::WeatherUnit;

use super::{
    GeocodingResponse, GeocodingResult, WeatherConfig, first_geocoding_result, weather_enabled,
};

#[test]
fn enables_weather_with_location_only() {
    let config = WeatherConfig {
        enabled: true,
        location: Some(String::from("Riga")),
        latitude: None,
        longitude: None,
        refresh_minutes: 15,
        unit: WeatherUnit::Celsius,
    };

    assert!(weather_enabled(&config));
}

#[test]
fn uses_first_geocoding_result_coordinates() {
    let payload = GeocodingResponse {
        results: vec![
            GeocodingResult {
                latitude: 56.9496,
                longitude: 24.1052,
            },
            GeocodingResult {
                latitude: 57.0,
                longitude: 24.2,
            },
        ],
    };

    assert_eq!(first_geocoding_result(payload), Some((56.9496, 24.1052)));
}

#[test]
fn returns_none_when_geocoding_has_no_results() {
    assert_eq!(
        first_geocoding_result(GeocodingResponse {
            results: Vec::new(),
        }),
        None
    );
}

#[test]
fn fresh_cache_waits_only_until_refresh_deadline_and_stale_or_future_cache_refreshes_now() {
    let config = veila_common::WeatherConfig::default();
    let mut snapshot = veila_common::WeatherSnapshot {
        temperature_celsius: 20,
        condition: veila_common::WeatherCondition::ClearDay,
        fetched_at_unix: 1000,
    };
    let interval = super::refresh_interval(&config);
    assert_eq!(
        super::cache_delay(&snapshot, &config, 1001),
        interval.saturating_sub(std::time::Duration::from_secs(1))
    );
    assert_eq!(
        super::cache_delay(&snapshot, &config, 1000 + interval.as_secs() as i64),
        std::time::Duration::ZERO
    );
    snapshot.fetched_at_unix = 2000;
    assert_eq!(
        super::cache_delay(&snapshot, &config, 1000),
        std::time::Duration::ZERO
    );
}
