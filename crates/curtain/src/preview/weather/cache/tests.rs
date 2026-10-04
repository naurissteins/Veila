use super::*;
use veila_common::WeatherCondition;

#[test]
fn prefers_cached_weather_snapshot_for_preview_when_available() {
    let cache_root =
        std::env::temp_dir().join(format!("veila-preview-weather-{}", std::process::id()));
    let weather_root = cache_root.join("veila").join("weather");
    fs::create_dir_all(&weather_root).expect("weather cache dir");

    let config = AppConfig::from_toml_str(
        r#"
            [weather]
            enabled = true
            location = "Seceda"
        "#,
    )
    .expect("config");

    let location_cache = preview_weather_location_cache_path(&weather_root, "Seceda");
    fs::write(
        &location_cache,
        serde_json::to_vec(&PreviewGeocodedLocationCache {
            latitude: 35.6762,
            longitude: 139.6503,
        })
        .expect("location cache"),
    )
    .expect("write location cache");

    let snapshot_cache =
        preview_weather_cache_path_for_coordinates(&weather_root, 35.6762, 139.6503);
    fs::write(
        &snapshot_cache,
        serde_json::to_vec(&WeatherSnapshot {
            temperature_celsius: 12,
            condition: WeatherCondition::Rain,
            fetched_at_unix: 123,
        })
        .expect("snapshot cache"),
    )
    .expect("write snapshot cache");

    let snapshot = load_cached_preview_weather_snapshot_from(&config, &weather_root, None)
        .expect("load cached preview snapshot");

    assert_eq!(
        snapshot,
        Some(WeatherSnapshot {
            temperature_celsius: 12,
            condition: WeatherCondition::Rain,
            fetched_at_unix: 123,
        })
    );

    fs::remove_file(location_cache).ok();
    fs::remove_file(snapshot_cache).ok();
    fs::remove_dir(weather_root).ok();
    fs::remove_dir(cache_root.join("veila")).ok();
    fs::remove_dir(cache_root).ok();
}
