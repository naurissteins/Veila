use anyhow::Result;

use crate::adapters::ipc;

pub(super) fn print_available_themes() -> Result<()> {
    for theme in veila_common::config::bundled_theme_names()? {
        println!("{theme}");
    }
    Ok(())
}

pub(super) fn print_current_theme(config_path: Option<&std::path::Path>) -> Result<()> {
    let theme = veila_common::config::active_theme_name(config_path)?;
    let source = veila_common::config::active_theme_source_path(config_path)?;

    println!("theme={}", theme.as_deref().unwrap_or("none"));
    println!(
        "source={}",
        source
            .as_deref()
            .map(|path| path.display().to_string())
            .as_deref()
            .unwrap_or("none")
    );

    Ok(())
}

pub(super) fn print_theme_source(theme: &str, config_path: Option<&std::path::Path>) -> Result<()> {
    let (path, raw) = veila_common::config::read_theme_source(config_path, theme)?;
    println!("theme={theme}");
    println!("source={}", path.display());
    println!();
    print!("{raw}");
    Ok(())
}

pub(super) async fn set_theme_and_reload(
    theme: &str,
    config_path: Option<&std::path::Path>,
    daemon_socket_path: &std::path::Path,
) -> Result<()> {
    let written_path = veila_common::config::set_theme_in_config(config_path, theme)?;
    println!("theme={theme}");
    println!("config={}", written_path.display());

    if !daemon_socket_path.exists() {
        println!("live_reload=not-running");
        return Ok(());
    }

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::ReloadConfig,
    )
    .await;

    match response {
        Ok(veila_common::ipc::DaemonControlResponse::Reloaded(status)) => {
            let daemon_config = status.config_path.as_deref().unwrap_or("defaults");
            println!("daemon_config={daemon_config}");
            println!(
                "daemon_config_matches={}",
                status
                    .config_path
                    .as_deref()
                    .is_some_and(|path| path == written_path.to_string_lossy())
            );
            println!("reload_source={}", status.reload_source);
            println!(
                "live_reload={}",
                match status.live_reload {
                    veila_common::ipc::LiveReloadStatus::NotActive => "not-active",
                    veila_common::ipc::LiveReloadStatus::Forwarded => "forwarded",
                }
            );
        }
        Ok(veila_common::ipc::DaemonControlResponse::Error { reason }) => {
            println!("live_reload=error");
            println!("reload_error={reason}");
        }
        Ok(_) => {
            println!("live_reload=error");
            println!("reload_error=unexpected-daemon-response");
        }
        Err(error) => {
            println!("live_reload=error");
            println!("reload_error={error}");
        }
    }

    Ok(())
}

pub(super) async fn unset_theme_and_reload(
    config_path: Option<&std::path::Path>,
    daemon_socket_path: &std::path::Path,
) -> Result<()> {
    let (written_path, changed) = veila_common::config::unset_theme_in_config(config_path)?;
    println!("config={}", written_path.display());
    println!("theme_removed={changed}");

    if !changed {
        println!("live_reload=not-needed");
        return Ok(());
    }

    if !daemon_socket_path.exists() {
        println!("live_reload=not-running");
        return Ok(());
    }

    let response = ipc::send_daemon_control_message(
        daemon_socket_path,
        &veila_common::ipc::DaemonControlMessage::ReloadConfig,
    )
    .await;

    match response {
        Ok(veila_common::ipc::DaemonControlResponse::Reloaded(status)) => {
            let daemon_config = status.config_path.as_deref().unwrap_or("defaults");
            println!("daemon_config={daemon_config}");
            println!(
                "daemon_config_matches={}",
                status
                    .config_path
                    .as_deref()
                    .is_some_and(|path| path == written_path.to_string_lossy())
            );
            println!("reload_source={}", status.reload_source);
            println!(
                "live_reload={}",
                match status.live_reload {
                    veila_common::ipc::LiveReloadStatus::NotActive => "not-active",
                    veila_common::ipc::LiveReloadStatus::Forwarded => "forwarded",
                }
            );
        }
        Ok(veila_common::ipc::DaemonControlResponse::Error { reason }) => {
            println!("live_reload=error");
            println!("reload_error={reason}");
        }
        Ok(_) => {
            println!("live_reload=error");
            println!("reload_error=unexpected-daemon-response");
        }
        Err(error) => {
            println!("live_reload=error");
            println!("reload_error={error}");
        }
    }

    Ok(())
}
