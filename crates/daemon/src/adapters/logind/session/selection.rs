use std::{error::Error, fmt};

use anyhow::{Context, anyhow};
use zbus::zvariant::OwnedObjectPath;

use super::super::proxy::session_proxy;

#[derive(Debug, Clone)]
pub(super) struct SessionSnapshot {
    pub uid: u32,
    pub active: bool,
    pub class: String,
    pub remote: bool,
    pub state: String,
    pub session_type: String,
    pub seat: String,
}

pub(super) struct SessionCandidate {
    pub id: String,
    pub path: OwnedObjectPath,
    pub snapshot: SessionSnapshot,
}

pub(super) async fn read_snapshot(
    conn: &zbus::Connection,
    path: &OwnedObjectPath,
) -> anyhow::Result<SessionSnapshot> {
    let proxy = session_proxy(conn, path).await?;
    Ok(SessionSnapshot {
        uid: proxy
            .user()
            .await
            .context("failed to read session owner")?
            .0,
        active: proxy
            .active()
            .await
            .context("failed to read session activity")?,
        class: proxy
            .class()
            .await
            .context("failed to read session class")?,
        remote: proxy
            .remote()
            .await
            .context("failed to read session locality")?,
        state: proxy
            .state()
            .await
            .context("failed to read session state")?,
        session_type: proxy
            .r#type()
            .await
            .context("failed to read session type")?,
        seat: proxy.seat().await.context("failed to read session seat")?.0,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum SessionRejection {
    OtherUser,
    NonInteractive,
    Remote,
    UnsupportedType,
    Unavailable,
}

impl fmt::Display for SessionRejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::OtherUser => "session belongs to another user",
            Self::NonInteractive => "session is not an interactive user session",
            Self::Remote => "session is remote",
            Self::UnsupportedType => "session is neither Wayland nor a local console",
            Self::Unavailable => "session is closing or unavailable",
        })
    }
}

impl Error for SessionRejection {}

pub(super) fn validate_session(
    snapshot: &SessionSnapshot,
    uid: u32,
) -> Result<(), SessionRejection> {
    if snapshot.uid != uid {
        return Err(SessionRejection::OtherUser);
    }
    if !matches!(
        snapshot.class.as_str(),
        "user" | "user-early" | "user-light" | "user-early-light"
    ) {
        return Err(SessionRejection::NonInteractive);
    }
    if snapshot.remote {
        return Err(SessionRejection::Remote);
    }
    // A compositor started from a console can retain logind's tty type.
    if !matches!(snapshot.session_type.as_str(), "wayland" | "tty") {
        return Err(SessionRejection::UnsupportedType);
    }
    if !matches!(snapshot.state.as_str(), "active" | "online") {
        return Err(SessionRejection::Unavailable);
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SessionRank {
    wayland: bool,
    active: bool,
    active_state: bool,
    has_seat: bool,
}

pub(super) fn session_selection_rank(snapshot: &SessionSnapshot) -> SessionRank {
    SessionRank {
        wayland: snapshot.session_type == "wayland",
        active: snapshot.active,
        active_state: snapshot.state == "active",
        has_seat: !snapshot.seat.is_empty(),
    }
}

pub(super) fn select_session(
    candidates: Vec<SessionCandidate>,
    uid: u32,
) -> anyhow::Result<SessionCandidate> {
    let mut best: Option<(SessionRank, SessionCandidate)> = None;
    let mut tied_ids = Vec::new();
    for candidate in candidates {
        if let Err(reason) = validate_session(&candidate.snapshot, uid) {
            tracing::debug!(session_id = %candidate.id, ?reason, snapshot = ?candidate.snapshot, "rejected logind session candidate");
            continue;
        }
        let rank = session_selection_rank(&candidate.snapshot);
        tracing::debug!(session_id = %candidate.id, ?rank, "ranked logind session candidate");
        match &best {
            Some((best_rank, _)) if rank < *best_rank => {}
            Some((best_rank, _)) if rank == *best_rank => tied_ids.push(candidate.id),
            _ => {
                tied_ids.clear();
                best = Some((rank, candidate));
            }
        }
    }
    let (_, selected) = best.ok_or_else(|| anyhow!("no eligible local user session found"))?;
    if !tied_ids.is_empty() {
        tied_ids.push(selected.id);
        return Err(anyhow!(
            "ambiguous local user sessions: {}; pass --session-id=<id>",
            tied_ids.join(", ")
        ));
    }
    Ok(selected)
}

#[cfg(test)]
mod tests;
