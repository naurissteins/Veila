use std::{sync::mpsc, thread};

use anyhow::{Context, Result, anyhow, bail};
use futures_util::StreamExt;
use smithay_client_toolkit::reexports::{
    client::{
        Connection, Dispatch, Proxy, QueueHandle,
        globals::{GlobalListContents, registry_queue_init},
        protocol::{wl_registry, wl_seat},
    },
    protocols::ext::idle_notify::v1::client::{ext_idle_notification_v1, ext_idle_notifier_v1},
};
use veila_common::ipc::LatencyReportMode;
use zbus::zvariant::OwnedFd;

use super::daemon::lock_running_daemon;
use crate::adapters::logind;

const DEFAULT_LOCK_AFTER_SECONDS: u64 = 300;

pub(super) async fn run_idle_monitor(
    daemon_socket_path: &std::path::Path,
    lock_after_seconds: Option<u64>,
    lock_before_sleep: bool,
) -> Result<()> {
    let lock_after_seconds = lock_after_seconds.unwrap_or(DEFAULT_LOCK_AFTER_SECONDS);
    let timeout_ms = timeout_millis(lock_after_seconds)?;
    let mut idle_events = spawn_wayland_idle_thread(timeout_ms)?;
    let sleep_connection = if lock_before_sleep {
        Some(logind::connect_system().await?)
    } else {
        None
    };
    let sleep_manager = match sleep_connection.as_ref() {
        Some(connection) => Some(
            logind::ManagerProxy::new(connection)
                .await
                .context("failed to create logind manager proxy for idle monitor")?,
        ),
        None => None,
    };
    let mut sleep_stream = match sleep_manager.as_ref() {
        Some(manager) => Some(
            manager
                .receive_prepare_for_sleep()
                .await
                .context("failed to subscribe to logind PrepareForSleep signal")?,
        ),
        None => None,
    };
    let mut sleep_inhibitor = match sleep_manager.as_ref() {
        Some(manager) => Some(acquire_sleep_delay_inhibitor(manager).await?),
        None => None,
    };

    println!("idle_monitor=true");
    println!("lock_after_seconds={lock_after_seconds}");
    println!("lock_before_sleep={lock_before_sleep}");

    loop {
        tokio::select! {
            event = idle_events.recv() => {
                match event {
                    Some(IdleMonitorEvent::Idled) => {
                        if let Err(error) = request_lock(daemon_socket_path, false, "idle").await {
                            tracing::warn!("failed to request idle lock: {error:#}");
                        }
                    }
                    None => bail!("Wayland idle monitor stopped unexpectedly"),
                }
            }
            signal = async {
                match sleep_stream.as_mut() {
                    Some(stream) => stream.next().await,
                    None => std::future::pending().await,
                }
            }, if sleep_stream.is_some() => {
                match signal {
                    Some(signal) => match signal.args() {
                        Ok(args) if *args.start() => {
                            match request_lock(daemon_socket_path, true, "sleep").await {
                                Ok(()) => {
                                    sleep_inhibitor.take();
                                    println!("sleep_lock_ready=true");
                                }
                                Err(error) => {
                                    tracing::warn!("failed to request sleep lock: {error:#}");
                                }
                            }
                        }
                        Ok(_) => {
                            if let Some(manager) = sleep_manager.as_ref() {
                                match acquire_sleep_delay_inhibitor(manager).await {
                                    Ok(inhibitor) => {
                                        sleep_inhibitor = Some(inhibitor);
                                        println!("sleep_inhibitor_armed=true");
                                    }
                                    Err(error) => {
                                        tracing::warn!("failed to rearm logind sleep inhibitor: {error:#}");
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!("failed to decode logind PrepareForSleep signal: {error}");
                        }
                    },
                    None => bail!("logind PrepareForSleep stream ended"),
                }
            }
        }
    }
}

async fn request_lock(
    daemon_socket_path: &std::path::Path,
    wait_ready: bool,
    source: &str,
) -> Result<()> {
    lock_running_daemon(
        daemon_socket_path,
        wait_ready,
        false,
        LatencyReportMode::Disabled,
    )
    .await?;
    println!("{source}_lock_requested=true");
    Ok(())
}

async fn acquire_sleep_delay_inhibitor(manager: &logind::ManagerProxy<'_>) -> Result<OwnedFd> {
    manager
        .inhibit("sleep", "Veila", "Lock the session before sleep", "delay")
        .await
        .context("failed to acquire logind sleep delay inhibitor")
}

fn spawn_wayland_idle_thread(
    timeout_ms: u32,
) -> Result<tokio::sync::mpsc::UnboundedReceiver<IdleMonitorEvent>> {
    let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();
    let (setup_sender, setup_receiver) = mpsc::sync_channel(1);

    thread::spawn(move || {
        let result = run_wayland_idle_loop(timeout_ms, event_sender, &setup_sender);
        if let Err(error) = result {
            let _ = setup_sender.send(Err(error.to_string()));
            tracing::warn!("Wayland idle monitor exited: {error:#}");
        }
    });

    match setup_receiver
        .recv()
        .context("idle monitor setup thread stopped before reporting readiness")?
    {
        Ok(()) => Ok(event_receiver),
        Err(error) => Err(anyhow!(error)),
    }
}

fn run_wayland_idle_loop(
    timeout_ms: u32,
    event_sender: tokio::sync::mpsc::UnboundedSender<IdleMonitorEvent>,
    setup_sender: &mpsc::SyncSender<std::result::Result<(), String>>,
) -> Result<()> {
    let connection =
        Connection::connect_to_env().context("failed to connect to Wayland display")?;
    let (globals, mut event_queue) =
        registry_queue_init(&connection).context("failed to enumerate Wayland globals")?;
    let queue_handle = event_queue.handle();

    let mut app = IdleApp::new(event_sender);
    let notifier = bind_idle_notifier(&globals, &queue_handle)?;
    let seat = bind_first_seat(&globals, &queue_handle)?;
    let _notification = notifier.get_idle_notification(timeout_ms, &seat, &queue_handle, ());
    connection
        .flush()
        .context("failed to flush idle notification request")?;
    let _ = setup_sender.send(Ok(()));

    loop {
        event_queue
            .blocking_dispatch(&mut app)
            .context("idle Wayland event dispatch failed")?;
    }
}

fn timeout_millis(seconds: u64) -> Result<u32> {
    let millis = seconds
        .checked_mul(1_000)
        .ok_or_else(|| anyhow!("--lock-after is too large"))?;
    u32::try_from(millis).map_err(|_| anyhow!("--lock-after is too large"))
}

fn bind_idle_notifier(
    globals: &smithay_client_toolkit::reexports::client::globals::GlobalList,
    queue_handle: &QueueHandle<IdleApp>,
) -> Result<ext_idle_notifier_v1::ExtIdleNotifierV1> {
    let advertised = globals.contents().with_list(|globals| {
        globals.iter().any(|global| {
            global.interface == ext_idle_notifier_v1::ExtIdleNotifierV1::interface().name
        })
    });
    if !advertised {
        bail!("compositor does not support ext-idle-notify-v1");
    }

    globals
        .bind(queue_handle, 1..=2, ())
        .context("failed to bind ext-idle-notify-v1")
}

fn bind_first_seat(
    globals: &smithay_client_toolkit::reexports::client::globals::GlobalList,
    queue_handle: &QueueHandle<IdleApp>,
) -> Result<wl_seat::WlSeat> {
    let advertised = globals.contents().with_list(|globals| {
        globals
            .iter()
            .any(|global| global.interface == wl_seat::WlSeat::interface().name)
    });
    if !advertised {
        bail!("compositor did not advertise a wl_seat");
    }

    globals
        .bind(queue_handle, 1..=9, ())
        .context("failed to bind wl_seat")
}

struct IdleApp {
    event_sender: tokio::sync::mpsc::UnboundedSender<IdleMonitorEvent>,
}

impl IdleApp {
    fn new(event_sender: tokio::sync::mpsc::UnboundedSender<IdleMonitorEvent>) -> Self {
        Self { event_sender }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdleMonitorEvent {
    Idled,
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
        match event {
            ext_idle_notification_v1::Event::Idled => {
                let _ = state.event_sender.send(IdleMonitorEvent::Idled);
            }
            ext_idle_notification_v1::Event::Resumed => {}
            _ => {}
        }
    }
}

smithay_client_toolkit::reexports::client::delegate_noop!(IdleApp: ignore wl_seat::WlSeat);

#[cfg(test)]
mod tests {
    use super::timeout_millis;

    #[test]
    fn timeout_millis_converts_seconds() {
        assert_eq!(timeout_millis(300).expect("timeout"), 300_000);
    }

    #[test]
    fn timeout_millis_rejects_overflow() {
        let error = timeout_millis(u64::MAX).expect_err("overflow should fail");
        assert!(error.to_string().contains("too large"));
    }
}
