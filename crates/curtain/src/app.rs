use std::io::{BufRead, BufReader, Read};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use calloop::channel::Event as ChannelEvent;
use calloop::signals::{Signal, Signals};
use smithay_client_toolkit::reexports::client::{Connection, globals::registry_queue_init};

use veila_common::{
    elapsed_ms, elapsed_us,
    ipc::{CurtainInitialSnapshots, IPC_MAX_LINE_BYTES, decode_message},
};

use crate::{CurtainOptions, preview, state::CurtainApp};

pub fn run(mut options: CurtainOptions) -> Result<()> {
    if options.preview_png.is_some() {
        return preview::render_preview(options);
    }

    if options.owner_gate {
        let snapshots = read_initial_snapshots(&mut BufReader::new(std::io::stdin()))?;
        options.weather_snapshot = snapshots.weather;
        options.battery_snapshot = snapshots.battery;
        options.now_playing_snapshot = snapshots.now_playing;
    }

    let startup_started_at = Instant::now();
    veila_renderer::text::start_font_warmup();

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
    let (mut app, event_sources) = CurtainApp::new(
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

    app.latency_timings.wayland_connect_ms = wayland_connect_elapsed_ms;
    app.latency_timings.wayland_connect_us = wayland_connect_elapsed_us;
    app.latency_timings.registry_ms = registry_elapsed_ms;
    app.latency_timings.registry_us = registry_elapsed_us;
    app.latency_timings.event_loop_ms = event_loop_elapsed_ms;
    app.latency_timings.event_loop_us = event_loop_elapsed_us;
    app.latency_timings.app_init_ms = app_init_elapsed_ms;
    app.latency_timings.app_init_us = app_init_elapsed_us;
    app.latency_timings.lock_request_ms = acquire_lock_elapsed_ms;
    app.latency_timings.lock_request_us = acquire_lock_elapsed_us;
    app.latency_timings.startup_prepared_ms = elapsed_ms(startup_started_at);
    app.latency_timings.startup_prepared_us = elapsed_us(startup_started_at);
    app.latency_timings.surface_count = app.lock_surfaces.len();

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
            app.request_exit_from_signal();
        })
        .context("failed to insert signal source into event loop")?;

    smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource::new(
        connection,
        event_queue,
    )
    .insert(loop_handle.clone())
    .context("failed to insert Wayland source into event loop")?;

    let control_queue = queue_handle.clone();
    loop_handle
        .insert_source(event_sources.control, move |(), _, app| {
            app.drain_control_events(&control_queue);
        })
        .map_err(|error| error.error)
        .context("failed to insert curtain control source into event loop")?;
    let auth_queue = queue_handle.clone();
    loop_handle
        .insert_source(event_sources.auth, move |event, _, app| {
            if let ChannelEvent::Msg(event) = event {
                app.drain_control_events(&auth_queue);
                if !app.exit_requested {
                    app.handle_auth_event(event, &auth_queue);
                }
            }
        })
        .map_err(|error| error.error)
        .context("failed to insert curtain authentication source into event loop")?;
    let background_queue = queue_handle.clone();
    loop_handle
        .insert_source(event_sources.background, move |event, _, app| {
            if let ChannelEvent::Msg(event) = event {
                app.drain_control_events(&background_queue);
                if !app.exit_requested {
                    app.handle_background_event(event, &background_queue);
                }
            }
        })
        .map_err(|error| error.error)
        .context("failed to insert curtain background source into event loop")?;

    while !app.can_stop() {
        event_loop
            .dispatch(app.next_event_timeout(), &mut app)
            .context("curtain event loop failed")?;
        app.advance_auth_watchdog(&queue_handle);
        app.advance_input_repeat(&queue_handle);
        app.advance_background_slideshow(&queue_handle);
        app.advance_output_power();
        app.advance_shm_trim();
        app.advance_animated_scene(&queue_handle);
        app.maybe_notify_startup();
        app.check_lock_deadline()?;
    }

    app.shutdown()?;

    if let Some(reason) = app.failure_reason() {
        bail!(reason.to_string());
    }

    Ok(())
}

fn read_initial_snapshots(reader: &mut impl BufRead) -> Result<CurtainInitialSnapshots> {
    let mut payload = Vec::new();
    let bytes = reader
        .take((IPC_MAX_LINE_BYTES + 1) as u64)
        .read_until(b'\n', &mut payload)
        .context("failed to read initial curtain snapshots")?;
    if bytes == 0 || bytes > IPC_MAX_LINE_BYTES || payload.last() != Some(&b'\n') {
        bail!("initial curtain snapshots exceed the IPC limit or are incomplete");
    }
    let snapshots = decode_message(std::str::from_utf8(&payload[..bytes - 1])?)
        .context("failed to decode initial curtain snapshots")?;
    let mut gate = [0_u8; 1];
    reader
        .read_exact(&mut gate)
        .context("daemon exited before publishing curtain ownership")?;
    if gate != [1] {
        bail!("invalid curtain ownership gate");
    }
    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use veila_common::{
        BatterySnapshot, NowPlayingSnapshot,
        ipc::{CurtainInitialSnapshots, IPC_MAX_LINE_BYTES, encode_message},
    };

    use super::read_initial_snapshots;

    #[test]
    fn reads_snapshots_only_after_valid_owner_gate() {
        let expected = CurtainInitialSnapshots {
            battery: Some(BatterySnapshot {
                percent: 84,
                charging: true,
            }),
            now_playing: Some(NowPlayingSnapshot {
                title: String::from("Track"),
                artist: Some(String::from("Artist")),
                artwork_path: None,
                fetched_at_unix: 7,
            }),
            ..Default::default()
        };
        let mut input = encode_message(&expected)
            .expect("snapshot JSON")
            .into_bytes();
        input.extend_from_slice(b"\n\x01");
        let actual = read_initial_snapshots(&mut BufReader::new(Cursor::new(input)))
            .expect("valid startup payload");
        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_payload_without_owner_gate() {
        let mut input = encode_message(&CurtainInitialSnapshots::default())
            .expect("snapshot JSON")
            .into_bytes();
        input.push(b'\n');
        assert!(read_initial_snapshots(&mut BufReader::new(Cursor::new(input))).is_err());
    }

    #[test]
    fn rejects_oversized_startup_payload() {
        let mut input = vec![b'x'; IPC_MAX_LINE_BYTES + 1];
        input.extend_from_slice(b"\n\x01");
        assert!(read_initial_snapshots(&mut BufReader::new(Cursor::new(input))).is_err());
    }
}
