use anyhow::Context;
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

pub async fn get_session_path(conn: &zbus::Connection) -> anyhow::Result<OwnedObjectPath> {
    let manager = ManagerProxy::new(conn)
        .await
        .context("failed to create logind manager proxy")?;

    if let Ok(session_id) = std::env::var("XDG_SESSION_ID")
        && !session_id.is_empty()
    {
        return manager
            .get_session(&session_id)
            .await
            .with_context(|| format!("failed to resolve logind session {session_id}"));
    }

    manager
        .get_session_by_pid(std::process::id())
        .await
        .context("failed to resolve logind session for the current process")
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
