mod config;
mod doctor;
mod help;
mod init;
mod logs;
mod runtime;
mod theme;

use anyhow::Result;

use crate::{ControlOptions, options::ControlCommand};
use config::print_config_validation;
use doctor::print_doctor_report;
use init::init_config;
use logs::print_logs;
use theme::{
    print_available_themes, print_current_theme, print_theme_source, set_theme_and_reload,
    unset_theme_and_reload,
};

const DAEMON_SERVICE: &str = "veila.service";

pub async fn run_control(options: ControlOptions) -> Result<()> {
    if options.help {
        help::print_control_help();
        return Ok(());
    }
    let config_path = options.config_path.as_deref();
    match options.command {
        Some(ControlCommand::Version) => {
            println!("Veila {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(ControlCommand::Init { theme, force }) => {
            init_config(config_path, theme.as_deref(), force)
        }
        Some(ControlCommand::ThemeCurrent) => print_current_theme(config_path),
        Some(ControlCommand::ThemePrint(theme)) => print_theme_source(&theme, config_path),
        Some(ControlCommand::ThemeList) => print_available_themes(),
        Some(ControlCommand::CheckConfig) => print_config_validation(config_path),
        Some(ControlCommand::Logs(logs)) => print_logs(config_path, &logs),
        Some(ControlCommand::ThemeSet(theme)) => set_theme_and_reload(&theme, config_path),
        Some(ControlCommand::ThemeUnset) => unset_theme_and_reload(config_path),
        Some(ControlCommand::Status) => runtime::print_status(),
        Some(ControlCommand::Doctor) => {
            print_doctor_report(config_path, None).await;
            Ok(())
        }
        Some(ControlCommand::Reload) => runtime::reload_config(),
        None => {
            help::print_control_help();
            Ok(())
        }
    }
}
