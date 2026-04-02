use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use calloop::signals::{Signal, Signals};
use smithay_client_toolkit::reexports::client::{Connection, globals::registry_queue_init};

use crate::{CurtainOptions, preview, state::CurtainApp};

pub fn run(options: CurtainOptions) -> Result<()> {
    if options.preview_png.is_some() {
        return preview::render_preview(options);
    }

    let startup_started_at = Instant::now();

    let wayland_connect_started_at = Instant::now();
    let connection =
        Connection::connect_to_env().context("failed to connect to Wayland display")?;
    let wayland_connect_elapsed_ms = elapsed_ms(wayland_connect_started_at);
    let wayland_connect_elapsed_us = elapsed_us(wayland_connect_started_at);

    let registry_started_at = Instant::now();
    let (globals, event_queue) =
        registry_queue_init(&connection).context("failed to enumerate Wayland globals")?;
    let registry_elapsed_ms = elapsed_ms(registry_started_at);
    let registry_elapsed_us = elapsed_us(registry_started_at);
    let queue_handle = event_queue.handle();

    let event_loop_started_at = Instant::now();
    let mut event_loop = smithay_client_toolkit::reexports::calloop::EventLoop::try_new()
        .context("failed to create curtain event loop")?;
    let event_loop_elapsed_ms = elapsed_ms(event_loop_started_at);
    let event_loop_elapsed_us = elapsed_us(event_loop_started_at);
    let loop_handle = event_loop.handle();

    let app_init_started_at = Instant::now();
    let mut app = CurtainApp::new(
        connection.clone(),
        &globals,
        &queue_handle,
        options,
        startup_started_at,
    )?;
    let app_init_elapsed_ms = elapsed_ms(app_init_started_at);
    let app_init_elapsed_us = elapsed_us(app_init_started_at);

    let acquire_lock_started_at = Instant::now();
    app.acquire_lock(&queue_handle)?;
    let acquire_lock_elapsed_ms = elapsed_ms(acquire_lock_started_at);
    let acquire_lock_elapsed_us = elapsed_us(acquire_lock_started_at);

    tracing::info!(
        wayland_connect_elapsed_ms,
        wayland_connect_elapsed_us,
        registry_elapsed_ms,
        registry_elapsed_us,
        event_loop_elapsed_ms,
        event_loop_elapsed_us,
        app_init_elapsed_ms,
        app_init_elapsed_us,
        acquire_lock_elapsed_ms,
        acquire_lock_elapsed_us,
        startup_prepared_elapsed_ms = elapsed_ms(startup_started_at),
        startup_prepared_elapsed_us = elapsed_us(startup_started_at),
        "curtain startup prepared"
    );

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
        app.drain_control_events(&queue_handle);
        app.drain_background_events(&queue_handle);
        app.drain_auth_events(&queue_handle);
        app.advance_animated_scene(&queue_handle);
        app.check_lock_deadline()?;
    }

    app.shutdown()?;

    if let Some(reason) = app.failure_reason() {
        bail!(reason.to_string());
    }

    Ok(())
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn elapsed_us(started_at: Instant) -> u64 {
    started_at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}
