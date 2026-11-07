mod channel;
mod events;
mod polling;
mod recovery;

use crate::{LockOptions, record::LockRecord};
use anyhow::Result;
use nix::sys::{
    signal::{SigSet, Signal},
    signalfd::{SfdFlags, SignalFd},
};
use serde::{Deserialize, Serialize};
use std::{os::unix::net::UnixStream, process::Child, time::Instant};
use veila_common::{
    AppConfig, elapsed_ms, elapsed_us,
    ipc::{LineAccumulator, LockLatencyReport},
    session::{self, frame},
};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Notification {
    pub code: i32,
    pub locked: bool,
    pub already_active: bool,
    pub report: Option<LockLatencyReport>,
}
pub(crate) struct Running {
    log: Option<crate::logging::SessionLog>,
    options: LockOptions,
    record: LockRecord,
    child: Option<Child>,
    channel: Option<UnixStream>,
    recovery: recovery::Recovery,
    notifications: Vec<std::fs::File>,
    idle_notify: Option<std::fs::File>,
    snapshots: Option<UnixStream>,
    snapshot_line: LineAccumulator,
    pending: Vec<u8>,
    written: usize,
    notified: bool,
    locked: bool,
    ready: bool,
    started: Instant,
    spawned: Instant,
    report: LockLatencyReport,
    config: AppConfig,
}

pub(crate) fn run(options: LockOptions) -> Result<i32> {
    let started = Instant::now();
    let mut notifications = Vec::new();
    let idle_notify = options
        .notify_fd
        .map(session::fd::take)
        .transpose()?
        .map(std::fs::File::from);
    for number in [options.background_fd].into_iter().flatten() {
        notifications.push(std::fs::File::from(session::fd::take(number)?));
    }
    for file in notifications.iter().chain(idle_notify.iter()) {
        use nix::fcntl::{FcntlArg, OFlag, fcntl};
        let flags = OFlag::from_bits_truncate(fcntl(file, FcntlArg::F_GETFL)?);
        fcntl(file, FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK))?;
    }
    let Some(record) = LockRecord::acquire(options.config.clone())? else {
        let reply = Notification {
            code: 0,
            locked: false,
            already_active: true,
            report: None,
        };
        for file in &mut notifications {
            let _ = frame::write(file, &reply);
        }
        println!("already_active=true");
        return Ok(0);
    };
    let config_started = Instant::now();
    let mut config = match AppConfig::load(options.config.as_deref()) {
        Ok(loaded) => loaded.config,
        Err(error) => {
            tracing::warn!(%error,"initial config unavailable; using safe defaults");
            AppConfig::default()
        }
    };
    if frame::encode(&config).is_err() {
        tracing::warn!("resolved config exceeds startup limit; using safe defaults");
        config = AppConfig::default();
    }
    let report = LockLatencyReport {
        daemon_config_load_ms: elapsed_ms(config_started),
        daemon_config_load_us: elapsed_us(config_started),
        ..Default::default()
    };
    let log = match crate::logging::open(&config) {
        Ok(file) => file,
        Err(error) => {
            tracing::warn!(%error,"optional log unavailable; using stderr");
            None
        }
    };
    if let Some(file) = &log {
        nix::unistd::dup2_stderr(&file.file)?;
    }
    let mut mask = SigSet::empty();
    for signal in [
        Signal::SIGINT,
        Signal::SIGHUP,
        Signal::SIGTERM,
        Signal::SIGCHLD,
    ] {
        mask.add(signal);
    }
    mask.thread_block()?;
    let signals = SignalFd::with_flags(&mask, SfdFlags::SFD_CLOEXEC | SfdFlags::SFD_NONBLOCK)?;
    let mut running = Running {
        log,
        options,
        record,
        child: None,
        channel: None,
        recovery: recovery::Recovery::default(),
        notifications,
        idle_notify,
        snapshots: None,
        snapshot_line: LineAccumulator::with_limit(session::MAX_FRAME_BYTES),
        pending: Vec::new(),
        written: 0,
        notified: false,
        locked: false,
        ready: false,
        started,
        spawned: Instant::now(),
        report,
        config,
    };
    running.launch()?;
    running.event_loop(signals)
}

impl Running {
    fn notify(&mut self, code: i32, already_active: bool) -> Result<()> {
        let reply = Notification {
            code,
            locked: self.locked,
            already_active,
            report: self
                .options
                .latency
                .is_enabled()
                .then(|| self.report.clone()),
        };
        for file in &mut self.notifications {
            if let Err(error) = frame::write(file, &reply) {
                tracing::warn!(%error,"lock notification receiver unavailable");
            }
        }
        self.notifications.clear();
        self.notified = true;
        if self.options.background_fd.is_none() {
            println!(
                "lock_ready={}\nalready_active={already_active}",
                self.locked
            );
            if self.options.latency.is_enabled() {
                crate::latency::print_latency_report(
                    &self.report,
                    self.options.latency.is_verbose(),
                );
            }
        }
        Ok(())
    }
}
