mod runtime;
mod services;
mod wayland;

use std::path::Path;

use super::local_build_info;

pub(super) async fn print_doctor_report(config_path: Option<&Path>, session_id: Option<&str>) {
    let mut summary = DoctorSummary::default();
    let local = local_build_info();

    println!("component={}", local.component);
    println!("version={}", local.version);
    println!("build_profile={}", local.build_profile);
    println!("target_os={}", local.target_os);
    println!("target_arch={}", local.target_arch);

    check_environment(&mut summary);
    check_config(&mut summary, config_path);
    check_themes(&mut summary);
    check_pam(&mut summary);
    runtime::check_daemon(&mut summary).await;
    services::check_idle_service(&mut summary);
    runtime::check_logind(&mut summary, session_id).await;
    wayland::check_wayland(&mut summary);

    println!("doctor={}", summary.overall());
    println!("doctor_errors={}", summary.errors);
    println!("doctor_warnings={}", summary.warnings);
}

fn check_environment(summary: &mut DoctorSummary) {
    let session_type = env_value("XDG_SESSION_TYPE");
    let wayland_display = env_value("WAYLAND_DISPLAY");
    let current_desktop = env_value("XDG_CURRENT_DESKTOP");
    let hyprland = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some();

    println!(
        "env.xdg_session_type={}",
        session_type.as_deref().unwrap_or("unset")
    );
    println!(
        "env.wayland_display={}",
        wayland_display.as_deref().unwrap_or("unset")
    );
    println!(
        "env.xdg_current_desktop={}",
        current_desktop.as_deref().unwrap_or("unset")
    );
    println!("env.hyprland_signature_present={hyprland}");

    if std::env::consts::OS != "linux" {
        summary.record(
            "environment",
            CheckStatus::Error,
            "Veila currently requires Linux session APIs",
        );
        return;
    }

    match (session_type.as_deref(), wayland_display.as_deref()) {
        (Some("wayland"), Some(_)) => summary.record(
            "environment",
            CheckStatus::Ok,
            "Wayland session variables are set",
        ),
        (Some(session_type), Some(_)) => summary.record(
            "environment",
            CheckStatus::Error,
            format!("XDG_SESSION_TYPE is {session_type}, expected wayland"),
        ),
        (Some("wayland"), None) => summary.record(
            "environment",
            CheckStatus::Error,
            "WAYLAND_DISPLAY is not set",
        ),
        _ => summary.record(
            "environment",
            CheckStatus::Error,
            "Wayland session variables are missing",
        ),
    }
}

fn check_config(summary: &mut DoctorSummary, config_path: Option<&Path>) {
    match veila_common::AppConfig::load(config_path) {
        Ok(loaded) => {
            println!(
                "config={}",
                loaded
                    .path
                    .as_deref()
                    .map(display_path)
                    .as_deref()
                    .unwrap_or("defaults")
            );
            summary.record("config", CheckStatus::Ok, "config loads successfully");
        }
        Err(error) => {
            println!(
                "config={}",
                config_path
                    .map(display_path)
                    .as_deref()
                    .unwrap_or("defaults")
            );
            summary.record(
                "config",
                CheckStatus::Error,
                format!("config failed to load: {error}"),
            );
        }
    }
}

fn check_themes(summary: &mut DoctorSummary) {
    match veila_common::config::bundled_theme_names() {
        Ok(themes) if themes.is_empty() => {
            println!("themes.count=0");
            summary.record("themes", CheckStatus::Error, "no bundled themes were found");
        }
        Ok(themes) => {
            println!("themes.count={}", themes.len());
            println!("themes.names={}", themes.join(","));
            summary.record("themes", CheckStatus::Ok, "bundled themes are available");
        }
        Err(error) => {
            println!("themes.count=unknown");
            summary.record(
                "themes",
                CheckStatus::Error,
                format!("failed to read bundled themes: {error}"),
            );
        }
    }
}

fn check_pam(summary: &mut DoctorSummary) {
    if let Some(service) = env_value("VEILA_PAM_SERVICE") {
        println!("pam.service={service}");
        summary.record(
            "pam",
            CheckStatus::Ok,
            "VEILA_PAM_SERVICE is set; external PAM service selection is active",
        );
        return;
    }

    let veila = Path::new("/etc/pam.d/veila");
    let system_auth = Path::new("/etc/pam.d/system-auth");
    println!("pam.service=veila");
    println!("pam.service_path={}", veila.display());

    if veila.exists() {
        summary.record("pam", CheckStatus::Ok, "Veila PAM service file exists");
    } else if system_auth.exists() {
        summary.record(
            "pam",
            CheckStatus::Warning,
            "Veila PAM service is missing; daemon will fall back to system-auth",
        );
    } else {
        summary.record(
            "pam",
            CheckStatus::Error,
            "no /etc/pam.d/veila service and no system-auth fallback found",
        );
    }
}

pub(super) fn env_value(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

#[derive(Debug, Default)]
pub(super) struct DoctorSummary {
    errors: u32,
    warnings: u32,
}

impl DoctorSummary {
    pub(super) fn record(&mut self, name: &str, status: CheckStatus, detail: impl AsRef<str>) {
        match status {
            CheckStatus::Ok => {}
            CheckStatus::Warning => self.warnings += 1,
            CheckStatus::Error => self.errors += 1,
        }

        println!("check.{name}={}", status.as_str());
        println!("check.{name}.detail={}", detail.as_ref());
    }

    fn overall(&self) -> &'static str {
        if self.errors > 0 {
            "error"
        } else if self.warnings > 0 {
            "warning"
        } else {
            "ok"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CheckStatus {
    Ok,
    Warning,
    Error,
}

impl CheckStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}
