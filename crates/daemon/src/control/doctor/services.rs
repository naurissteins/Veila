use crate::adapters::user_units;

use super::{
    super::DAEMON_SERVICE,
    CheckStatus, DoctorSummary,
    systemd::{ShowError, UnitState, print_unit_state, show_user_unit},
};

const LEGACY_DAEMON_SERVICE: &str = "veilad.service";

pub(super) fn check_daemon_service(summary: &mut DoctorSummary) {
    check_legacy_daemon_enablement(summary);

    let Some(state) = show_unit(DAEMON_SERVICE, "daemon_service", summary) else {
        return;
    };

    match (
        state.load.as_deref(),
        state.active.as_deref(),
        state.unit_file.as_deref(),
    ) {
        (Some("loaded"), Some("active"), _) => summary.record(
            "daemon_service",
            CheckStatus::Ok,
            "veila.service is loaded and active",
        ),
        (Some("loaded"), Some(active), Some("enabled")) => summary.record(
            "daemon_service",
            CheckStatus::Warning,
            format!("veila.service is enabled but its active state is {active}"),
        ),
        (Some("loaded"), _, _) => summary.record(
            "daemon_service",
            CheckStatus::Ok,
            "veila.service is installed but not running; fine when your compositor starts `veila daemon`",
        ),
        (Some("not-found"), _, _) => summary.record(
            "daemon_service",
            CheckStatus::Warning,
            "veila.service is not installed in the user service manager",
        ),
        _ => summary.record(
            "daemon_service",
            CheckStatus::Warning,
            "veila.service state could not be determined",
        ),
    }
}

fn check_legacy_daemon_enablement(summary: &mut DoctorSummary) {
    let links = user_units::enablement_links(&user_units::config_dir(), LEGACY_DAEMON_SERVICE);
    if links.is_empty() {
        println!("daemon_service.legacy_enablement=none");
        return;
    }

    for link in &links {
        println!("daemon_service.legacy_enablement={}", link.display());
    }
    summary.record(
        "daemon_service_legacy",
        CheckStatus::Warning,
        format!(
            "{LEGACY_DAEMON_SERVICE} is still enabled under its old name; it keeps working for now, but migrate with `rm {}` and `systemctl --user enable {DAEMON_SERVICE}`",
            links
                .iter()
                .map(|link| link.display().to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ),
    );
}

fn show_unit(unit: &str, prefix: &str, summary: &mut DoctorSummary) -> Option<UnitState> {
    match show_user_unit(unit) {
        Ok(properties) => {
            println!("{prefix}.systemctl=ok");
            Some(print_unit_state(&properties, prefix))
        }
        Err(ShowError::Unavailable(detail)) => {
            println!("{prefix}.systemctl=unavailable");
            summary.record(prefix, CheckStatus::Warning, detail);
            None
        }
        Err(ShowError::Failed(detail)) => {
            println!("{prefix}.systemctl=error");
            summary.record(prefix, CheckStatus::Warning, detail);
            None
        }
    }
}
