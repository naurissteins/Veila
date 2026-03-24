use anyhow::{Context, anyhow};
use zbus::{proxy, zvariant::OwnedObjectPath};

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
pub trait Manager {
    fn get_session(&self, session_id: &str) -> zbus::Result<OwnedObjectPath>;
    fn get_session_by_pid(&self, pid: u32) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1"
)]
pub trait Session {
    #[zbus(signal)]
    fn lock(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn unlock(&self) -> zbus::Result<()>;

    fn set_locked_hint(&self, locked: bool) -> zbus::Result<()>;
}

pub async fn connect_system() -> anyhow::Result<zbus::Connection> {
    zbus::Connection::system()
        .await
        .context("failed to connect to the system D-Bus")
}

pub async fn get_session_path(
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

pub async fn session_proxy<'a>(
    conn: &'a zbus::Connection,
    session_path: &'a OwnedObjectPath,
) -> anyhow::Result<SessionProxy<'a>> {
    SessionProxy::builder(conn)
        .path(session_path.as_str())
        .context("invalid logind session object path")?
        .build()
        .await
        .context("failed to create logind session proxy")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionLookupCandidate {
    SessionId { source: &'static str, value: String },
    Pid,
}

fn session_lookup_candidates(session_id_override: Option<&str>) -> Vec<SessionLookupCandidate> {
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

fn normalized_session_id(value: Option<&str>) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::{SessionLookupCandidate, normalized_session_id, session_lookup_candidates};

    #[test]
    fn normalizes_session_ids() {
        assert_eq!(normalized_session_id(Some(" c2 ")).as_deref(), Some("c2"));
        assert_eq!(normalized_session_id(Some("   ")), None);
    }

    #[test]
    fn prefers_cli_session_id_before_pid_lookup() {
        let candidates = session_lookup_candidates(Some("c2"));

        assert_eq!(
            candidates.first(),
            Some(&SessionLookupCandidate::SessionId {
                source: "cli",
                value: "c2".to_string(),
            })
        );
        assert!(matches!(
            candidates.last(),
            Some(SessionLookupCandidate::Pid)
        ));
    }
}
