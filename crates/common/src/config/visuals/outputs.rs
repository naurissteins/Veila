use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutputVisualConfig {
    #[serde(default)]
    pub ui_mode: Option<OutputUiMode>,
    #[serde(default)]
    pub ui_output: Option<String>,
}

impl Default for OutputVisualConfig {
    fn default() -> Self {
        Self {
            ui_mode: Some(OutputUiMode::All),
            ui_output: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputUiMode {
    #[default]
    All,
    Single,
}

impl super::VisualConfig {
    pub fn output_ui_mode(&self) -> OutputUiMode {
        self.outputs
            .as_ref()
            .and_then(|outputs| outputs.ui_mode)
            .unwrap_or_default()
    }

    pub fn ui_output_name(&self) -> Option<&str> {
        self.outputs
            .as_ref()
            .and_then(|outputs| outputs.ui_output.as_deref())
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }
}
