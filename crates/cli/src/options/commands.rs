use super::LogOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlCommand {
    Version,
    Status,
    Doctor,
    CheckConfig,
    Init { theme: Option<String>, force: bool },
    Reload,
    Logs(LogOptions),
    ThemeList,
    ThemeCurrent,
    ThemePrint(String),
    ThemeSet(String),
    ThemeUnset,
}
