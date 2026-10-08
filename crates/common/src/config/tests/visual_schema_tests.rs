use super::*;

const RETIRED_KEYS: &[&str] = &[
    "input_font_family",
    "input_font_weight",
    "input_font_style",
    "input_font_size",
    "input_border",
    "input_width",
    "input_height",
    "input_radius",
    "input_border_width",
    "input_mask_color",
    "avatar_background_color",
    "avatar_size",
    "avatar_placeholder_padding",
    "avatar_icon_color",
    "avatar_ring_color",
    "avatar_ring_width",
    "username_color",
    "username_font_size",
    "clock_font_family",
    "clock_font_weight",
    "clock_font_style",
    "clock_style",
    "clock_format",
    "clock_meridiem_font_size",
    "clock_meridiem_x",
    "clock_meridiem_y",
    "clock_color",
    "clock_font_size",
    "date_color",
    "date_font_size",
    "placeholder_color",
    "eye_icon_color",
    "keyboard_color",
    "keyboard_background_size",
    "keyboard_size",
    "battery_color",
    "battery_background_color",
    "battery_background_size",
    "battery_size",
    "status_color",
    "foreground",
    "muted",
    "pending",
    "rejected",
    "avatar_radius",
    "panel",
    "panel_border",
];

#[test]
fn retired_visual_keys_are_unknown_in_every_config_source() {
    let root = std::env::temp_dir().join(format!("veila-retired-visuals-{}", std::process::id()));
    fs::create_dir_all(root.join("themes")).expect("theme dir");
    let config_path = root.join("config.toml");
    let include_path = root.join("colors.toml");
    let theme_path = root.join("themes/custom.toml");
    // Use invalid legacy values to prove loading no longer interprets these keys.
    let retired = format!(
        "[visuals]\n{}",
        RETIRED_KEYS
            .iter()
            .map(|key| format!("{key} = false\n"))
            .collect::<String>()
    );
    for (source, target) in [
        ("config", &config_path),
        ("include", &include_path),
        ("theme", &theme_path),
    ] {
        fs::write(&config_path, "theme = 'custom'\ninclude = 'colors.toml'\n")
            .expect("main config");
        fs::write(&include_path, "").expect("include");
        fs::write(&theme_path, "").expect("theme");
        let prefix = if source == "config" {
            "theme = 'custom'\ninclude = 'colors.toml'\n"
        } else {
            ""
        };
        fs::write(target, format!("{prefix}{retired}")).expect("retired keys");
        let report = AppConfig::validate(Some(&config_path)).expect("load ignores unknown keys");
        let mut actual = report
            .issues
            .iter()
            .map(|issue| {
                assert_eq!(issue.source.as_str(), source);
                assert_eq!(&issue.path, target);
                issue.key_path.clone()
            })
            .collect::<Vec<_>>();
        actual.sort();
        let mut expected = RETIRED_KEYS
            .iter()
            .map(|key| format!("visuals.{key}"))
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(actual, expected);
    }
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn retired_flat_keys_do_not_override_nested_defaults() {
    let config = AppConfig::from_toml_str_with_theme_support(
        "[visuals]\nclock_font_size = 1\ninput_radius = 99\nforeground = '#010203'\n",
        None,
    )
    .expect("unknown keys do not block lock configuration");
    let defaults = AppConfig::from_default_layers().expect("default layer");
    assert_eq!(config.visuals, defaults.visuals);
}

#[test]
fn scalar_input_is_rejected_in_favor_of_the_nested_section() {
    for input in ["'#010203'", "[1, 2, 3]", "[1, 2, 3, 4]"] {
        assert!(AppConfig::from_toml_str(&format!("[visuals]\ninput = {input}\n")).is_err());
        assert!(
            AppConfig::from_toml_str_with_theme_support(
                &format!("[visuals]\ninput = {input}\n"),
                None,
            )
            .is_err()
        );
    }
}

#[test]
fn serialized_visual_config_contains_only_nested_sections() {
    let config = AppConfig::default();
    let value = toml::Value::try_from(&config).expect("serialization");
    let visuals = value["visuals"].as_table().expect("visual sections");
    assert!(
        visuals
            .keys()
            .all(|key| !RETIRED_KEYS.contains(&key.as_str()))
    );
    assert!(visuals["input"].is_table());
    let roundtrip = AppConfig::from_toml_str(&toml::to_string(&config).expect("TOML"))
        .expect("roundtrip config");
    assert_eq!(roundtrip, config);
}

#[test]
fn partial_nested_input_retains_defaults_and_zero_values() {
    let config =
        AppConfig::from_toml_str("[visuals.input]\nradius = 0\nborder_width = 0\nwidth = 0\n")
            .expect("nested input");
    assert_eq!(config.visuals.input_font_family(), Some("Google Sans Flex"));
    assert_eq!(config.visuals.input_height(), Some(54));
    assert_eq!(config.visuals.input_radius(), 0);
    assert_eq!(config.visuals.input_border_width(), Some(0));
    assert_eq!(config.visuals.input_width(), Some(0));
}
