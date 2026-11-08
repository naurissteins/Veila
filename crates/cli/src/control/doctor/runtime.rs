use crate::adapters::logind;

use super::{CheckStatus, DoctorSummary};

pub(super) fn check_processes(summary: &mut DoctorSummary) {
    match veila_session::runtime_status() {
        Ok(status) => {
            println!(
                "lock.state={}\nidle.running={}",
                status.state,
                status.idle.is_some()
            );
            summary.record(
                "processes",
                CheckStatus::Ok,
                "per-display process records inspected; the idle watcher is optional",
            );
        }
        Err(error) => summary.record(
            "processes",
            CheckStatus::Warning,
            format!("could not inspect process records: {error}"),
        ),
    }
}

pub(super) async fn check_logind(summary: &mut DoctorSummary, session_id: Option<&str>) {
    let connection = match logind::connect_system().await {
        Ok(connection) => connection,
        Err(error) => {
            summary.record(
                "logind",
                CheckStatus::Error,
                format!("failed to connect to system D-Bus/logind: {error}"),
            );
            return;
        }
    };

    let session_path = match logind::get_session_path(&connection, session_id).await {
        Ok(path) => path,
        Err(error) => {
            summary.record(
                "logind",
                CheckStatus::Error,
                format!("failed to resolve logind session: {error}"),
            );
            return;
        }
    };

    println!("logind.session={session_path}");
    let session = match logind::session_proxy(&connection, &session_path).await {
        Ok(session) => session,
        Err(error) => {
            summary.record(
                "logind",
                CheckStatus::Error,
                format!("failed to create logind session proxy: {error}"),
            );
            return;
        }
    };

    let active = session.active().await.unwrap_or(false);
    let state = session
        .state()
        .await
        .unwrap_or_else(|_| "unknown".to_string());
    let session_type = session
        .r#type()
        .await
        .unwrap_or_else(|_| "unknown".to_string());

    println!("logind.active={active}");
    println!("logind.state={state}");
    println!("logind.type={session_type}");

    if active && session_type == "wayland" {
        summary.record(
            "logind",
            CheckStatus::Ok,
            "active Wayland logind session resolved",
        );
    } else if session_type == "wayland" {
        summary.record(
            "logind",
            CheckStatus::Warning,
            "Wayland logind session resolved, but it is not active",
        );
    } else {
        summary.record(
            "logind",
            CheckStatus::Error,
            format!("resolved logind session type is {session_type}, expected wayland"),
        );
    }
}
