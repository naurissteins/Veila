use crate::adapters::logind;

use super::{CheckStatus, DoctorSummary};

pub(super) fn check_processes(summary: &mut DoctorSummary) -> bool {
    match veila_session::runtime_status() {
        Ok(status) => {
            println!(
                "lock.state={}\nlock.supervisor_running={}\nlock.curtain_running={}\nidle.running={}",
                status.state,
                status.supervisor.is_some(),
                status.curtain.is_some(),
                status.idle.is_some()
            );
            let (verdict, detail) = process_verdict(&status);
            summary.record("processes", verdict, detail);
            status.idle.is_some()
        }
        Err(error) => {
            summary.record(
                "processes",
                CheckStatus::Warning,
                format!("could not inspect process records: {error}"),
            );
            false
        }
    }
}

fn process_verdict(status: &veila_session::RuntimeStatus) -> (CheckStatus, &'static str) {
    match (
        status.state,
        status.supervisor.is_some(),
        status.curtain.is_some(),
    ) {
        ("unknown" | "abnormal" | "refused", _, _) => (
            CheckStatus::Warning,
            "lock record needs inspection; use the lock log for details",
        ),
        ("locked", false, true) => (
            CheckStatus::Warning,
            "curtain survives without its supervisor; password PAM remains available, but integrations are unavailable",
        ),
        ("locked", _, false) => (
            CheckStatus::Warning,
            "confirmed lock has no live curtain; inspect supervisor recovery logs",
        ),
        _ => (
            CheckStatus::Ok,
            "per-display process records inspected; manual locking needs no watcher",
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_watcher_is_healthy_but_surviving_curtain_reports_lost_integrations() {
        let mut status = veila_session::RuntimeStatus {
            state: "unlocked",
            ..Default::default()
        };
        assert_eq!(process_verdict(&status).0, CheckStatus::Ok);
        status.state = "locked";
        status.curtain = Some(1);
        let verdict = process_verdict(&status);
        assert_eq!(verdict.0, CheckStatus::Warning);
        assert!(verdict.1.contains("PAM remains available"));
        status.curtain = None;
        assert!(process_verdict(&status).1.contains("recovery"));
        status.state = "unknown";
        assert_eq!(process_verdict(&status).0, CheckStatus::Warning);
    }
}
