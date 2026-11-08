use crate::{
    LockOptions,
    record::{LockRecord, start_ticks},
};
use anyhow::{Context, Result};
use std::{
    io::Write,
    os::{
        fd::AsRawFd,
        unix::{net::UnixStream, process::CommandExt},
    },
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use veila_common::{
    AppConfig,
    ipc::CurtainInitialSnapshots,
    session::{fd, frame},
};

pub(crate) fn curtain(
    options: &LockOptions,
    config: &AppConfig,
    record: &mut LockRecord,
    log: Option<std::fs::File>,
    report: &mut veila_common::ipc::LockLatencyReport,
) -> Result<(Child, UnixStream)> {
    let socket_started = Instant::now();
    let (channel, inherited) = UnixStream::pair()?;
    report.socket_setup_ms = veila_common::elapsed_ms(socket_started);
    report.socket_setup_us = veila_common::elapsed_us(socket_started);
    let mut command = Command::new("/proc/self/exe");
    command
        .process_group(0)
        .arg("__curtain")
        .arg("--lock")
        .arg(format!("--ipc-fd={}", inherited.as_raw_fd()))
        .arg(format!("--lock-fd={}", record.file.as_raw_fd()))
        .stdin(Stdio::piped())
        .stdout(Stdio::null());
    if options.emergency {
        command.arg("--force-emergency-ui");
    }
    if options.latency.is_enabled() {
        command.arg(if options.latency.is_verbose() {
            "--latency-report=verbose"
        } else {
            "--latency-report"
        });
    }
    if record.recovered {
        command.arg(format!(
            "--initial-backoff-ms={}",
            config.lock.auth_backoff_max_seconds.saturating_mul(1000)
        ));
    }
    if let Some(log) = log {
        command.stderr(Stdio::from(log));
    }
    let config_frame = frame::encode(config)?;
    let snapshot_frame = frame::encode(&CurtainInitialSnapshots::default())?;
    fd::inheritable(&inherited, true)?;
    fd::inheritable(&record.file, true)?;
    let child = command.spawn();
    let channel_flags = fd::inheritable(&inherited, false);
    let record_flags = fd::inheritable(&record.file, false);
    let mut child = child.context("failed to spawn curtain")?;
    let startup = (|| -> Result<()> {
        channel_flags?;
        record_flags?;
        channel.set_nonblocking(true)?;
        record.value.curtain_pid = Some(child.id());
        record.value.curtain_start_ticks = Some(start_ticks(child.id())?);
        record.save()?;
        let mut input = child.stdin.take().context("missing curtain startup pipe")?;
        input.write_all(&config_frame)?;
        input.write_all(&snapshot_frame)?;
        Ok(())
    })();
    if let Err(error) = startup {
        let _ = child.kill();
        // Never start a replacement unless the failed spawn is known to have exited.
        if child.wait().is_err() {
            return Err(anyhow::Error::new(UnresolvedChild).context(error));
        }
        return Err(error);
    }
    // The inherited endpoint is dropped here; supervisor EOF can now reach the curtain.
    drop(inherited);
    Ok((child, channel))
}

#[derive(Debug)]
pub(crate) struct UnresolvedChild;
impl std::fmt::Display for UnresolvedChild {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cannot prove failed curtain exited; refusing concurrent recovery")
    }
}
impl std::error::Error for UnresolvedChild {}

pub(crate) fn background(options: LockOptions, mut args: Vec<String>) -> Result<i32> {
    use std::io::BufReader;
    let notify = options.notify_fd.map(fd::take).transpose()?;
    let (mut parent, inherited) = UnixStream::pair()?;
    let mut skip = false;
    args.retain(|arg| {
        if skip {
            skip = false;
            return false;
        }
        if arg == "--notify-fd" {
            skip = true;
            return false;
        }
        arg != "--daemonize" && !arg.starts_with("--notify-fd=")
    });
    let mut command = Command::new("/proc/self/exe");
    command
        .arg("lock")
        .args(args.into_iter().skip(1))
        .arg(format!("--background-fd={}", inherited.as_raw_fd()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    fd::inheritable(&inherited, true)?;
    if let Some(fd) = &notify {
        fd::inheritable(fd, true)?;
        command.args(["--notify-fd", &fd.as_raw_fd().to_string()]);
    }
    // Use a re-exec while single-threaded instead of forking a process with logging workers.
    let result = command.spawn();
    fd::inheritable(&inherited, false)?;
    let mut child = result?;
    drop(inherited);
    drop(notify);
    parent.set_read_timeout(Some(Duration::from_secs(130)))?;
    let reply: crate::app::Notification = frame::read(&mut BufReader::new(&mut parent))?
        .context("supervisor closed before lock confirmation")?;
    let _ = child.try_wait();
    if let Some(report) = reply.report {
        crate::latency::print_latency_report(&report, options.latency.is_verbose());
    }
    println!(
        "lock_ready={}\nalready_active={}",
        reply.locked, reply.already_active
    );
    Ok(reply.code)
}
