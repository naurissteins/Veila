use super::super::weather::preview_weather_hidden;
use super::*;

#[test]
fn preview_username_prefers_override() {
    let options = CurtainOptions {
        preview_username: Some(String::from("guest")),
        ..CurtainOptions::default()
    };
    let config = AppConfig::from_toml_str(
        r#"
            [visuals.username]
            text = "ns"
        "#,
    )
    .expect("config");

    assert_eq!(
        preview_username(&options, &config),
        Some(String::from("guest"))
    );
}

#[test]
fn preview_hide_widgets_removes_keyboard_label() {
    let options = CurtainOptions {
        preview_hide_widgets: true,
        ..CurtainOptions::default()
    };

    assert_eq!(preview_keyboard_layout_label(&options), None);
}

#[test]
fn preview_hide_keyboard_label_removes_only_keyboard_label() {
    let options = CurtainOptions {
        preview_hide_keyboard_label: true,
        ..CurtainOptions::default()
    };
    let mut config = AppConfig::default();
    config.weather.enabled = true;

    assert_eq!(preview_keyboard_layout_label(&options), None);
    assert!(!preview_weather_hidden(&options, &config));
    assert!(!preview_battery_hidden(&options));
    assert!(!preview_now_playing_hidden(&options));
}

#[test]
fn preview_hide_weather_hides_only_weather() {
    let options = CurtainOptions {
        preview_hide_weather: true,
        ..CurtainOptions::default()
    };
    let mut config = AppConfig::default();
    config.weather.enabled = true;

    assert!(preview_weather_hidden(&options, &config));
    assert!(!preview_battery_hidden(&options));
    assert!(!preview_now_playing_hidden(&options));
}

#[test]
fn preview_hide_battery_hides_only_battery() {
    let options = CurtainOptions {
        preview_hide_battery: true,
        ..CurtainOptions::default()
    };
    let mut config = AppConfig::default();
    config.weather.enabled = true;

    assert!(!preview_weather_hidden(&options, &config));
    assert!(preview_battery_hidden(&options));
    assert!(!preview_now_playing_hidden(&options));
}

#[test]
fn preview_hide_now_playing_hides_only_now_playing() {
    let options = CurtainOptions {
        preview_hide_now_playing: true,
        ..CurtainOptions::default()
    };
    let mut config = AppConfig::default();
    config.weather.enabled = true;

    assert!(!preview_weather_hidden(&options, &config));
    assert!(!preview_battery_hidden(&options));
    assert!(preview_now_playing_hidden(&options));
}

#[test]
fn preview_battery_override_uses_requested_percent_and_charging() {
    let options = CurtainOptions {
        preview_battery_percent: Some(91),
        preview_battery_charging: Some(true),
        ..CurtainOptions::default()
    };
    let config = AppConfig::default();

    assert_eq!(
        preview_battery_snapshot(&options, &config),
        Some(BatterySnapshot {
            percent: 91,
            charging: true,
        })
    );
}
