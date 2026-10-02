use anyhow::{Context, anyhow};
use nix::unistd::getuid;
use zbus::zvariant::OwnedObjectPath;

use super::proxy::ManagerProxy;
use selection::{SessionCandidate, read_snapshot, select_session, validate_session};

mod selection;

pub(crate) async fn get_session_path(
    conn: &zbus::Connection,
    session_id_override: Option<&str>,
) -> anyhow::Result<OwnedObjectPath> {
    let manager = ManagerProxy::new(conn)
        .await
        .context("failed to create logind manager proxy")?;
    let environment_session_id = std::env::var("XDG_SESSION_ID").ok();
    let candidates =
        session_lookup_candidates(session_id_override, environment_session_id.as_deref());
    let pid = std::process::id();
    let uid = getuid().as_raw();
    let mut failures = Vec::new();

    for candidate in candidates {
        tracing::debug!(?candidate, "resolving logind session");
        match resolve_candidate(conn, &manager, &candidate, pid, uid).await {
            Ok(path) => {
                tracing::debug!(?candidate, session = %path, "resolved logind session");
                return Ok(path);
            }
            Err(error) => {
                tracing::debug!(?candidate, error = %format!("{error:#}"), "rejected logind session lookup");
                failures.push(format!("{candidate:?}: {error:#}"));
            }
        }
    }

    Err(anyhow!(build_resolution_error(
        session_id_override,
        pid,
        &failures
    )))
}

async fn resolve_candidate(
    conn: &zbus::Connection,
    manager: &ManagerProxy<'_>,
    candidate: &SessionLookupCandidate,
    pid: u32,
    uid: u32,
) -> anyhow::Result<OwnedObjectPath> {
    let path = match candidate {
        SessionLookupCandidate::Explicit(value) | SessionLookupCandidate::Environment(value) => {
            manager.get_session(value).await?
        }
        SessionLookupCandidate::Pid => manager.get_session_by_pid(pid).await?,
        SessionLookupCandidate::ListByUid => {
            return select_session_from_uid_list(conn, manager, uid).await;
        }
    };
    let snapshot = read_snapshot(conn, &path).await?;
    validate_session(&snapshot, uid)
        .with_context(|| format!("session {path} rejected: {snapshot:?}"))?;
    Ok(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionLookupCandidate {
    Explicit(String),
    Environment(String),
    Pid,
    ListByUid,
}

pub(super) fn session_lookup_candidates(
    session_id_override: Option<&str>,
    environment_session_id: Option<&str>,
) -> Vec<SessionLookupCandidate> {
    // An explicit target must never silently resolve to a different session.
    if let Some(value) = normalized_session_id(session_id_override) {
        return vec![SessionLookupCandidate::Explicit(value)];
    }
    let mut candidates = Vec::new();
    if let Some(value) = normalized_session_id(environment_session_id) {
        candidates.push(SessionLookupCandidate::Environment(value));
    }
    candidates.extend([
        SessionLookupCandidate::Pid,
        SessionLookupCandidate::ListByUid,
    ]);
    candidates
}

pub(super) fn normalized_session_id(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

async fn select_session_from_uid_list(
    conn: &zbus::Connection,
    manager: &ManagerProxy<'_>,
    uid: u32,
) -> anyhow::Result<OwnedObjectPath> {
    let mut candidates = Vec::new();
    for (id, session_uid, _, _, path) in manager.list_sessions().await? {
        if session_uid != uid {
            continue;
        }
        match read_snapshot(conn, &path).await {
            Ok(snapshot) => candidates.push(SessionCandidate { id, path, snapshot }),
            Err(error) => {
                tracing::debug!(session = %path, %error, "could not inspect logind session candidate")
            }
        }
    }
    let selected = select_session(candidates, uid)?;
    tracing::debug!(session_id = %selected.id, session = %selected.path, "selected local user logind session");
    Ok(selected.path)
}

fn build_resolution_error(
    session_id_override: Option<&str>,
    pid: u32,
    failures: &[String],
) -> String {
    let xdg_session_id = std::env::var("XDG_SESSION_ID").ok();
    let xdg_session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let attempts = failures.join("; ");
    format!(
        "failed to resolve a local user logind session. attempts: {attempts}. context: \
pid={pid}, cli_session_id={}, xdg_session_id={}, xdg_session_type={}, wayland_display={}. \
Run `veila daemon` from your desktop session terminal or pass --session-id=<id> for the intended local user session.",
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
