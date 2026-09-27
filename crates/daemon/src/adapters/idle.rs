use std::{
    io::ErrorKind,
    os::fd::{AsFd, AsRawFd, RawFd},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use smithay_client_toolkit::reexports::{
    client::{
        Connection, Dispatch, EventQueue, Proxy, QueueHandle,
        backend::WaylandError,
        globals::{GlobalList, GlobalListContents, registry_queue_init},
        protocol::{wl_registry, wl_seat},
    },
    protocols::ext::idle_notify::v1::client::{ext_idle_notification_v1, ext_idle_notifier_v1},
};
use tokio::{
    io::{Interest, unix::AsyncFd},
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinHandle,
};

/// Compositor idle notifications for one timeout; dropping it disconnects from Wayland.
pub struct IdleNotifier {
    task: JoinHandle<()>,
    idled: UnboundedReceiver<()>,
}

impl IdleNotifier {
    pub fn spawn(timeout: Duration) -> Self {
        let (sender, idled) = unbounded_channel();
        let task = tokio::spawn(async move {
            if let Err(error) = run(timeout, sender).await {
                tracing::warn!("idle locking is unavailable: {error:#}");
            }
        });
        Self { task, idled }
    }

    /// Resolves on each idle notification; returns `None` once the notifier has stopped.
    pub async fn idled(&mut self) -> Option<()> {
        self.idled.recv().await
    }
}

impl Drop for IdleNotifier {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct ConnectionFd(Connection);

impl AsRawFd for ConnectionFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_fd().as_raw_fd()
    }
}

struct IdleSession {
    connection: Connection,
    queue: EventQueue<IdleApp>,
    _notification: ext_idle_notification_v1::ExtIdleNotificationV1,
}

async fn run(timeout: Duration, sender: UnboundedSender<()>) -> Result<()> {
    let timeout_ms = u32::try_from(timeout.as_millis()).context("idle timeout is too large")?;
    let session = tokio::task::spawn_blocking(move || connect(timeout_ms))
        .await
        .context("idle Wayland setup task failed")??;
    let IdleSession {
        connection,
        mut queue,
        _notification,
    } = session;
    let fd = AsyncFd::with_interest(ConnectionFd(connection.clone()), Interest::READABLE)
        .context("failed to watch the Wayland connection")?;
    let mut app = IdleApp { sender };
    tracing::info!(timeout_seconds = timeout.as_secs(), "idle locking armed");

    loop {
        queue
            .dispatch_pending(&mut app)
            .context("idle Wayland event dispatch failed")?;
        if app.sender.is_closed() {
            return Ok(());
        }
        flush(&connection)?;

        let mut ready = fd
            .readable()
            .await
            .context("failed to wait for Wayland events")?;
        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        match guard.read() {
            Ok(_) => {}
            Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => {
                ready.clear_ready();
            }
            Err(error) => return Err(anyhow!(error).context("failed to read Wayland events")),
        }
    }
}

fn connect(timeout_ms: u32) -> Result<IdleSession> {
    let connection =
        Connection::connect_to_env().context("failed to connect to Wayland display")?;
    let (globals, queue) =
        registry_queue_init(&connection).context("failed to enumerate Wayland globals")?;
    let handle = queue.handle();
    let notifier = bind_idle_notifier(&globals, &handle)?;
    let seat = bind_first_seat(&globals, &handle)?;
    let notification = notifier.get_idle_notification(timeout_ms, &seat, &handle, ());
    flush(&connection)?;

    Ok(IdleSession {
        connection,
        queue,
        _notification: notification,
    })
}

fn flush(connection: &Connection) -> Result<()> {
    match connection.flush() {
        Ok(()) => Ok(()),
        Err(WaylandError::Io(error)) if error.kind() == ErrorKind::WouldBlock => Ok(()),
        Err(error) => Err(anyhow!(error).context("failed to flush Wayland requests")),
    }
}

fn bind_idle_notifier(
    globals: &GlobalList,
    handle: &QueueHandle<IdleApp>,
) -> Result<ext_idle_notifier_v1::ExtIdleNotifierV1> {
    if !advertises(
        globals,
        ext_idle_notifier_v1::ExtIdleNotifierV1::interface().name,
    ) {
        bail!("compositor does not support ext-idle-notify-v1");
    }

    globals
        .bind(handle, 1..=2, ())
        .context("failed to bind ext-idle-notify-v1")
}

fn bind_first_seat(globals: &GlobalList, handle: &QueueHandle<IdleApp>) -> Result<wl_seat::WlSeat> {
    if !advertises(globals, wl_seat::WlSeat::interface().name) {
        bail!("compositor did not advertise a wl_seat");
    }

    globals
        .bind(handle, 1..=9, ())
        .context("failed to bind wl_seat")
}

fn advertises(globals: &GlobalList, interface: &str) -> bool {
    globals
        .contents()
        .with_list(|globals| globals.iter().any(|global| global.interface == interface))
}

struct IdleApp {
    sender: UnboundedSender<()>,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for IdleApp {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: <wl_registry::WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_idle_notifier_v1::ExtIdleNotifierV1, ()> for IdleApp {
    fn event(
        _: &mut Self,
        _: &ext_idle_notifier_v1::ExtIdleNotifierV1,
        _: ext_idle_notifier_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_idle_notification_v1::ExtIdleNotificationV1, ()> for IdleApp {
    fn event(
        state: &mut Self,
        _: &ext_idle_notification_v1::ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if matches!(event, ext_idle_notification_v1::Event::Idled) {
            let _ = state.sender.send(());
        }
    }
}

smithay_client_toolkit::reexports::client::delegate_noop!(IdleApp: ignore wl_seat::WlSeat);
