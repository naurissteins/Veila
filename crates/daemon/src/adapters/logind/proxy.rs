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

pub(crate) async fn connect_system() -> anyhow::Result<zbus::Connection> {
    zbus::Connection::system()
        .await
        .context("failed to connect to the system D-Bus")
}

pub(crate) async fn session_proxy<'a>(
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
