use crate::adapters::{ipc, logind};

use super::{CheckStatus, DoctorSummary};

pub(super) async fn check_daemon(summary: &mut DoctorSummary) {
    let daemon_socket = match ipc::daemon_socket_path() {
        Ok(path) => path,
        Err(error) => {
            summary.record(
                "daemon",
                CheckStatus::Warning,
                format!("failed to resolve daemon socket path: {error}"),
            );
            return;
        }
    };
    println!("daemon.socket={}", daemon_socket.display());

    if !daemon_socket.exists() {
        summary.record(
            "daemon",
            CheckStatus::Warning,
            "veilad is not running at the expected socket",
        );
        return;
    }

    let response = ipc::send_daemon_control_message(
        &daemon_socket,
        &veila_common::ipc::DaemonControlMessage::Health,
    )
    .await;

    match response {
        Ok(veila_common::ipc::DaemonControlResponse::Health(health)) => {
            println!("daemon.version={}", health.version);
            println!("daemon.target_os={}", health.target_os);
            println!("daemon.target_arch={}", health.target_arch);
            summary.record(
                "daemon",
                CheckStatus::Ok,
                "running daemon responded to health check",
            );
        }
        Ok(_) => summary.record(
            "daemon",
            CheckStatus::Warning,
            "daemon returned an unexpected health response",
        ),
        Err(error) => summary.record(
            "daemon",
            CheckStatus::Warning,
            format!("daemon socket exists but health check failed: {error}"),
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
