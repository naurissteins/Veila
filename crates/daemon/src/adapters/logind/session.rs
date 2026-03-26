use anyhow::{Context, anyhow};
use zbus::zvariant::OwnedObjectPath;

use super::proxy::ManagerProxy;

pub(crate) async fn get_session_path(
    conn: &zbus::Connection,
    session_id_override: Option<&str>,
) -> anyhow::Result<OwnedObjectPath> {
    let manager = ManagerProxy::new(conn)
        .await
        .context("failed to create logind manager proxy")?;
    let candidates = session_lookup_candidates(session_id_override);
    let pid = std::process::id();
    let mut failures = Vec::new();

    for candidate in candidates {
        match candidate {
            SessionLookupCandidate::SessionId { source, value } => {
                tracing::debug!(source, session_id = %value, "resolving logind session by session id");
                match manager.get_session(&value).await {
                    Ok(path) => {
                        tracing::debug!(source, session = %path, "resolved logind session");
                        return Ok(path);
                    }
                    Err(error) => {
                        failures.push(format!("{source} session id {value}: {error}"));
                    }
                }
            }
            SessionLookupCandidate::Pid => {
                tracing::debug!(pid, "resolving logind session by current process pid");
                match manager.get_session_by_pid(pid).await {
                    Ok(path) => {
                        tracing::debug!(session = %path, pid, "resolved logind session");
                        return Ok(path);
                    }
                    Err(error) => {
                        failures.push(format!("current pid {pid}: {error}"));
                    }
                }
            }
        }
    }

    Err(anyhow!(build_resolution_error(
        session_id_override,
        pid,
        &failures,
    )))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionLookupCandidate {
    SessionId { source: &'static str, value: String },
    Pid,
}

pub(super) fn session_lookup_candidates(
    session_id_override: Option<&str>,
) -> Vec<SessionLookupCandidate> {
    let mut candidates = Vec::new();

    if let Some(session_id) = normalized_session_id(session_id_override) {
        candidates.push(SessionLookupCandidate::SessionId {
            source: "cli",
            value: session_id,
        });
    }

    if let Some(session_id) = normalized_session_id(std::env::var("XDG_SESSION_ID").ok().as_deref())
        && !candidates.iter().any(|candidate| {
            matches!(
                candidate,
                SessionLookupCandidate::SessionId { value, .. } if value == &session_id
            )
        })
    {
        candidates.push(SessionLookupCandidate::SessionId {
            source: "env",
            value: session_id,
        });
    }

    candidates.push(SessionLookupCandidate::Pid);
    candidates
}

pub(super) fn normalized_session_id(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn build_resolution_error(
    session_id_override: Option<&str>,
    pid: u32,
    failures: &[String],
) -> String {
    let xdg_session_id = std::env::var("XDG_SESSION_ID").ok();
    let xdg_session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let hyprland_present = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some();
    let attempts = failures.join("; ");

    format!(
        "failed to resolve logind session. attempts: {attempts}. context: \
pid={pid}, cli_session_id={}, xdg_session_id={}, xdg_session_type={}, wayland_display={}, hyprland_signature_present={hyprland_present}. \
Run veilad from your normal interactive session terminal, ensure logind authorizes the caller, or pass --session-id=<id> explicitly.",
        display_option(session_id_override),
        display_option(xdg_session_id.as_deref()),
        display_option(xdg_session_type.as_deref()),
        display_option(wayland_display.as_deref()),
    )
}

fn display_option(value: Option<&str>) -> &str {
    value
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unset")
}
