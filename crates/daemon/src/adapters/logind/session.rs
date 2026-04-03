use anyhow::{Context, anyhow};
use nix::unistd::getuid;
use zbus::zvariant::OwnedObjectPath;

use super::proxy::{ManagerProxy, session_proxy};

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
            SessionLookupCandidate::ListByUid { uid } => {
                tracing::debug!(uid, "resolving logind session by listing sessions for uid");
                match manager.list_sessions().await {
                    Ok(sessions) => {
                        let sessions = sessions
                            .into_iter()
                            .filter(|(_, session_uid, _, _, _)| *session_uid == uid)
                            .collect::<Vec<_>>();
                        if let Some((session_id, path)) =
                            select_session_from_uid_list(conn, sessions).await
                        {
                            tracing::debug!(session = %path, session_id, "resolved logind session via list");
                            return Ok(path);
                        }
                        failures.push(format!("list-sessions uid={uid}: no session found"));
                    }
                    Err(error) => {
                        failures.push(format!("list-sessions uid={uid}: {error}"));
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
    ListByUid { uid: u32 },
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
    candidates.push(SessionLookupCandidate::ListByUid {
        uid: getuid().as_raw(),
    });
    candidates
}

pub(super) fn normalized_session_id(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

async fn select_session_from_uid_list(
    conn: &zbus::Connection,
    sessions: Vec<(String, u32, String, String, OwnedObjectPath)>,
) -> Option<(String, OwnedObjectPath)> {
    if sessions.len() == 1 {
        let selected = sessions
            .into_iter()
            .next()
            .map(|(session_id, _, _, _, path)| (session_id, path));
        if let Some((session_id, path)) = selected.as_ref() {
            tracing::debug!(
                session_id,
                session = %path,
                "selected only same-uid logind session candidate"
            );
        }
        return selected;
    }

    let preferred_type = std::env::var("XDG_SESSION_TYPE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let mut best: Option<(i32, String, OwnedObjectPath)> = None;

    for (session_id, _, _, seat, path) in sessions {
        let Ok(proxy) = session_proxy(conn, &path).await else {
            continue;
        };

        let snapshot = SessionSelectionSnapshot {
            active: proxy.active().await.unwrap_or(false),
            class: proxy.class().await.unwrap_or_default(),
            remote: proxy.remote().await.unwrap_or(false),
            state: proxy.state().await.unwrap_or_default(),
            session_type: proxy.r#type().await.unwrap_or_default(),
            seat,
        };
        let score = session_selection_score(&snapshot, preferred_type.as_deref());
        tracing::debug!(
            session_id,
            session = %path,
            score,
            active = snapshot.active,
            state = snapshot.state,
            class = snapshot.class,
            session_type = snapshot.session_type,
            remote = snapshot.remote,
            seat = snapshot.seat,
            preferred_type = preferred_type.as_deref().unwrap_or("unset"),
            "scored same-uid logind session fallback candidate"
        );
        match &best {
            Some((best_score, ..)) if score <= *best_score => {}
            _ => best = Some((score, session_id, path)),
        }
    }

    if let Some((score, session_id, path)) = best.as_ref() {
        tracing::debug!(
            session_id,
            session = %path,
            score,
            "selected same-uid logind session fallback candidate"
        );
    }

    best.map(|(_, session_id, path)| (session_id, path))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionSelectionSnapshot {
    pub(super) active: bool,
    pub(super) class: String,
    pub(super) remote: bool,
    pub(super) state: String,
    pub(super) session_type: String,
    pub(super) seat: String,
}

pub(super) fn session_selection_score(
    snapshot: &SessionSelectionSnapshot,
    preferred_type: Option<&str>,
) -> i32 {
    let mut score = 0;

    if snapshot.active {
        score += 100;
    }
    if snapshot.state == "active" {
        score += 80;
    }
    if snapshot.class == "user" {
        score += 60;
    }
    if !snapshot.seat.trim().is_empty() {
        score += 20;
    }
    if let Some(preferred_type) = preferred_type
        && snapshot.session_type == preferred_type
    {
        score += 120;
    }
    if snapshot.remote {
        score -= 200;
    }

    score
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
