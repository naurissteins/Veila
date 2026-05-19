mod visuals;

use std::path::Path;

use toml::Value;

use super::{ConfigValidationIssue, ConfigValidationSourceKind};
use visuals::VISUALS;

#[derive(Debug, Clone, Copy)]
pub(super) struct KeyRule {
    name: &'static str,
    schema: Schema,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Schema {
    Any,
    Table(&'static [KeyRule]),
    ArrayTable(&'static [KeyRule]),
}

pub(super) fn unknown_key_issues(
    source: ConfigValidationSourceKind,
    path: &Path,
    value: &Value,
) -> Vec<ConfigValidationIssue> {
    let mut issues = Vec::new();
    validate_table(source, path, value, TOP_LEVEL, "", &mut issues);
    issues
}

fn validate_table(
    source: ConfigValidationSourceKind,
    path: &Path,
    value: &Value,
    rules: &'static [KeyRule],
    prefix: &str,
    issues: &mut Vec<ConfigValidationIssue>,
) {
    let Some(table) = value.as_table() else {
        return;
    };

    for (key, value) in table {
        let key_path = child_path(prefix, key);
        let Some(rule) = rules.iter().find(|rule| rule.name == key) else {
            issues.push(ConfigValidationIssue {
                source,
                path: path.to_path_buf(),
                key_path,
                message: format!("unknown config key `{key}`"),
            });
            continue;
        };
        validate_schema(source, path, value, rule.schema, &key_path, issues);
    }
}

fn validate_schema(
    source: ConfigValidationSourceKind,
    path: &Path,
    value: &Value,
    schema: Schema,
    key_path: &str,
    issues: &mut Vec<ConfigValidationIssue>,
) {
    match schema {
        Schema::Any => {}
        Schema::Table(rules) => validate_table(source, path, value, rules, key_path, issues),
        Schema::ArrayTable(rules) => {
            let Some(values) = value.as_array() else {
                return;
            };
            for (index, value) in values.iter().enumerate() {
                validate_table(
                    source,
                    path,
                    value,
                    rules,
                    &format!("{key_path}[{index}]"),
                    issues,
                );
            }
        }
    }
}

fn child_path(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    }
}

const TOP_LEVEL: &[KeyRule] = &[
    key("theme", Schema::Any),
    key("include", Schema::Any),
    key("background", Schema::Table(BACKGROUND)),
    key("lock", Schema::Table(LOCK)),
    key("fingerprint", Schema::Table(FINGERPRINT)),
    key("battery", Schema::Table(BATTERY)),
    key("now_playing", Schema::Table(NOW_PLAYING)),
    key("weather", Schema::Table(WEATHER)),
    key("visuals", Schema::Table(VISUALS)),
];

const BACKGROUND: &[KeyRule] = &[
    key("mode", Schema::Any),
    key("path", Schema::Any),
    key("outputs", Schema::ArrayTable(BACKGROUND_OUTPUT)),
    key("slideshow", Schema::Table(BACKGROUND_SLIDESHOW)),
    key("color", Schema::Any),
    key("scaling", Schema::Any),
    key("gradient", Schema::Table(BACKGROUND_GRADIENT)),
    key("layered", Schema::Table(BACKGROUND_LAYERED)),
    key("radial", Schema::Table(BACKGROUND_RADIAL)),
    key("blur_strength", Schema::Any),
    key("dim_strength", Schema::Any),
    key("tint", Schema::Any),
];

const BACKGROUND_OUTPUT: &[KeyRule] = &[key("name", Schema::Any), key("path", Schema::Any)];

const BACKGROUND_SLIDESHOW: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("directory", Schema::Any),
    key("files", Schema::Any),
    key("order", Schema::Any),
    key("mode", Schema::Any),
    key("change_every_seconds", Schema::Any),
];

const BACKGROUND_GRADIENT: &[KeyRule] = &[
    key("top_left", Schema::Any),
    key("top_right", Schema::Any),
    key("bottom_left", Schema::Any),
    key("bottom_right", Schema::Any),
];

const BACKGROUND_RADIAL: &[KeyRule] = &[
    key("center", Schema::Any),
    key("edge", Schema::Any),
    key("center_x", Schema::Any),
    key("center_y", Schema::Any),
    key("radius", Schema::Any),
];

const BACKGROUND_LAYERED: &[KeyRule] = &[
    key("base", Schema::Table(BACKGROUND_LAYERED_BASE)),
    key("blobs", Schema::ArrayTable(BACKGROUND_LAYERED_BLOB)),
];

const BACKGROUND_LAYERED_BASE: &[KeyRule] = &[
    key("mode", Schema::Any),
    key("color", Schema::Any),
    key("gradient", Schema::Table(BACKGROUND_GRADIENT)),
    key("radial", Schema::Table(BACKGROUND_RADIAL)),
];

const BACKGROUND_LAYERED_BLOB: &[KeyRule] = &[
    key("color", Schema::Any),
    key("opacity", Schema::Any),
    key("x", Schema::Any),
    key("y", Schema::Any),
    key("size", Schema::Any),
];

const LOCK: &[KeyRule] = &[
    key("acquire_timeout_seconds", Schema::Any),
    key("auto_reload_config", Schema::Any),
    key("auto_reload_debounce_ms", Schema::Any),
    key("log_to_file", Schema::Any),
    key("log_file_path", Schema::Any),
    key("auth_backoff_base_ms", Schema::Any),
    key("auth_backoff_max_seconds", Schema::Any),
    key("hide_cursor", Schema::Any),
    key("allow_empty_password", Schema::Any),
    key("screen_off_seconds", Schema::Any),
    key("power_off_secondary_outputs", Schema::Any),
    key("suspend_seconds", Schema::Any),
    key("suspend_only_on_battery", Schema::Any),
    key("skip_suspend_while_media_playing", Schema::Any),
    key("avatar_path", Schema::Any),
];

const BATTERY: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("refresh_seconds", Schema::Any),
    key("mock_percent", Schema::Any),
    key("mock_charging", Schema::Any),
];

const FINGERPRINT: &[KeyRule] = &[key("enabled", Schema::Any)];

const NOW_PLAYING: &[KeyRule] = &[
    key("include_players", Schema::Any),
    key("exclude_players", Schema::Any),
];

const WEATHER: &[KeyRule] = &[
    key("enabled", Schema::Any),
    key("location", Schema::Any),
    key("latitude", Schema::Any),
    key("longitude", Schema::Any),
    key("refresh_minutes", Schema::Any),
    key("unit", Schema::Any),
];

pub(super) const fn key(name: &'static str, schema: Schema) -> KeyRule {
    KeyRule { name, schema }
}
