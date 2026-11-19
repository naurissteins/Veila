use std::{fs, process::Command};

use veila_common::{WeatherCondition, WeatherUnit};

use super::{
    GeocodingResponse, GeocodingResult, OpenMeteoCurrent, WeatherConfig, finish_snapshot,
    first_geocoding_result, weather_enabled,
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
fn fetched_snapshot_survives_an_unwritable_cache() {
    const CHILD: &str = "VEILA_TEST_UNWRITABLE_WEATHER_CACHE";
    if std::env::var_os(CHILD).is_some() {
        let current = OpenMeteoCurrent {
            temperature_2m: 17.6,
            weather_code: 61,
            is_day: 1,
        };
        let snapshot = finish_snapshot(current, 56.9496, 24.1052);
        assert_eq!(
            (snapshot.temperature_celsius, snapshot.condition),
            (18, WeatherCondition::Rain)
        );
        return;
    }

    let cache_home =
        std::env::temp_dir().join(format!("veila-weather-cache-{}", std::process::id()));
    // A regular file cannot contain cache entries, even when the tests run as root.
    fs::File::create_new(&cache_home).expect("block cache directory creation");
    let output = Command::new(std::env::current_exe().expect("test executable"))
        .args([
            "--exact",
            "services::weather::tests::fetched_snapshot_survives_an_unwritable_cache",
        ])
        .env(CHILD, "1")
        .env("XDG_CACHE_HOME", &cache_home)
        .output();
    fs::remove_file(cache_home).expect("remove blocked cache path");
    let output = output.expect("isolated cache test process");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
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
