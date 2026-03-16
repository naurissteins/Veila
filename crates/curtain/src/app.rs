use std::time::Duration;

use anyhow::{Context, Result, bail};
use calloop::signals::{Signal, Signals};
use smithay_client_toolkit::reexports::client::{Connection, globals::registry_queue_init};

use crate::{CurtainOptions, state::CurtainApp};

pub fn run(options: CurtainOptions) -> Result<()> {
    let connection =
        Connection::connect_to_env().context("failed to connect to Wayland display")?;
    let (globals, event_queue) =
        registry_queue_init(&connection).context("failed to enumerate Wayland globals")?;
    let queue_handle = event_queue.handle();
    let mut event_loop = smithay_client_toolkit::reexports::calloop::EventLoop::try_new()
        .context("failed to create curtain event loop")?;
    let loop_handle = event_loop.handle();
    let mut app = CurtainApp::new(connection.clone(), &globals, &queue_handle, options)?;

    app.acquire_lock(&queue_handle)?;

    let signals = Signals::new(&[Signal::SIGINT, Signal::SIGTERM])
        .context("failed to register signal source")?;
    loop_handle
        .insert_source(signals, |event, _, app: &mut CurtainApp| {
            tracing::info!(?event, "termination requested");
            app.request_exit();
        })
        .context("failed to insert signal source into event loop")?;

    smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource::new(
        connection,
        event_queue,
    )
    .insert(loop_handle)
    .context("failed to insert Wayland source into event loop")?;

    while !app.can_stop() {
        event_loop
            .dispatch(Duration::from_millis(250), &mut app)
            .context("curtain event loop failed")?;
        app.drain_auth_events(&queue_handle);
        app.check_lock_deadline()?;
    }

    app.shutdown()?;

    if let Some(reason) = app.failure_reason() {
        bail!(reason.to_string());
    }

    Ok(())
}
