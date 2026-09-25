use std::{collections::HashMap, process::Command};

const UNIT_PROPERTIES: [&str; 5] = [
    "LoadState",
    "ActiveState",
    "SubState",
    "UnitFileState",
    "FragmentPath",
];

pub(super) enum ShowError {
    Unavailable(String),
    Failed(String),
}

pub(super) struct UnitState {
    pub(super) load: Option<String>,
    pub(super) active: Option<String>,
    pub(super) unit_file: Option<String>,
}

pub(super) fn show_user_unit(unit: &str) -> Result<HashMap<String, String>, ShowError> {
    let mut command = Command::new("systemctl");
    command.args(["--user", "show", unit, "--no-pager"]);
    for property in UNIT_PROPERTIES {
        command.arg(format!("--property={property}"));
    }

    let output = command.output().map_err(|error| {
        ShowError::Unavailable(format!("failed to run systemctl --user: {error}"))
    })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ShowError::Failed(if detail.is_empty() {
            format!("systemctl --user show {unit} failed")
        } else {
            format!("systemctl --user show {unit} failed: {detail}")
        }));
    }

    Ok(parse_systemctl_show(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

pub(super) fn print_unit_state(properties: &HashMap<String, String>, prefix: &str) -> UnitState {
    let load = print_property(properties, "LoadState", prefix, "load_state");
    let active = print_property(properties, "ActiveState", prefix, "active_state");
    print_property(properties, "SubState", prefix, "sub_state");
    let unit_file = print_property(properties, "UnitFileState", prefix, "unit_file_state");
    print_property(properties, "FragmentPath", prefix, "fragment_path");

    UnitState {
        load,
        active,
        unit_file,
    }
}

fn print_property(
    properties: &HashMap<String, String>,
    key: &str,
    prefix: &str,
    output_key: &str,
) -> Option<String> {
    let value = properties
        .get(key)
        .filter(|value| !value.is_empty())
        .cloned();
    println!(
        "{prefix}.{output_key}={}",
        value.as_deref().unwrap_or("unknown")
    );
    value
}

fn parse_systemctl_show(output: &str) -> HashMap<String, String> {
    let mut properties = HashMap::new();
    for line in output.lines() {
        if let Some((key, value)) = line.split_once('=') {
            properties.insert(key.to_string(), value.to_string());
        }
    }
    properties
}
