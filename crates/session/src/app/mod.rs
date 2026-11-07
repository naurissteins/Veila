mod channel;
mod events;

use crate::{
    LockOptions,
    record::{LockRecord, State},
};
use anyhow::Result;
use nix::{
    poll::{PollFd, PollFlags, poll},
    sys::{
        signal::{SigSet, Signal},
        signalfd::{SfdFlags, SignalFd},
    },
};
use serde::{Deserialize, Serialize};
use std::{
    os::{fd::AsFd, unix::net::UnixStream},
    process::Child,
    time::{Duration, Instant},
};
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
    child: Child,
    channel: UnixStream,
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
    let Some(mut record) = LockRecord::acquire(options.config.clone())? else {
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
    let mut report = LockLatencyReport {
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
    let spawn_started = Instant::now();
    let (child, channel) = crate::spawn::curtain(
        &options,
        &config,
        &mut record,
        log.as_ref().map(|log| log.file.try_clone()).transpose()?,
        &mut report,
    )?;
    report.curtain_spawn_ms = elapsed_ms(spawn_started);
    report.curtain_spawn_us = elapsed_us(spawn_started);
    let mut running = Running {
        log,
        options,
        record,
        child,
        channel,
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
    running.event_loop(signals)
}

impl Running {
    fn event_loop(&mut self, signals: SignalFd) -> Result<i32> {
        let mut line = LineAccumulator::with_limit(session::MAX_FRAME_BYTES);
        let mut connected = true;
        loop {
            if let Some(status) = self.child.try_wait()? {
                if connected {
                    self.read_events(&mut line, &mut connected)?;
                }
                let code = status.code().unwrap_or(session::EXIT_ABNORMAL);
                let code = if code == 0 && !self.locked {
                    session::EXIT_ABNORMAL
                } else {
                    code
                };
                self.record.value.state = if code == 0 {
                    State::Unlocked
                } else if code == session::EXIT_REFUSED && !self.locked {
                    State::Refused
                } else {
                    State::Abnormal
                };
                self.record.save()?;
                if !self.notified {
                    self.notify(code.max(1), false)?;
                }
                return Ok(code);
            }
            // readiness failure reports an error without stopping an already-secure, usable curtain
            let timeout = Duration::from_secs(self.config.lock.acquire_timeout_seconds.max(1))
                .saturating_add(Duration::from_secs(2));
            if !self.notified && self.spawned.elapsed() > timeout {
                self.notify(1, false)?;
            }
            let mut fds = vec![PollFd::new(signals.as_fd(), PollFlags::POLLIN)];
            if connected {
                fds.push(PollFd::new(
                    self.channel.as_fd(),
                    PollFlags::POLLIN
                        | if self.pending.is_empty() {
                            PollFlags::empty()
                        } else {
                            PollFlags::POLLOUT
                        },
                ));
            }
            let snapshot_index = fds.len();
            if let Some(reader) = &self.snapshots {
                fds.push(PollFd::new(reader.as_fd(), PollFlags::POLLIN));
            }
            let wait = if !self.notified {
                nix::poll::PollTimeout::try_from(timeout.saturating_sub(self.spawned.elapsed()))
                    .unwrap_or(nix::poll::PollTimeout::MAX)
            } else if self.log.is_some() {
                nix::poll::PollTimeout::from(15_000u16)
            } else {
                nix::poll::PollTimeout::NONE
            };
            match poll(&mut fds, wait) {
                Ok(_) => {}
                Err(nix::errno::Errno::EINTR) => continue,
                Err(e) => return Err(e.into()),
            }
            let socket_ready = connected
                && fds.get(1).is_some_and(|f| {
                    f.revents().is_some_and(|r| {
                        r.intersects(PollFlags::POLLIN | PollFlags::POLLHUP | PollFlags::POLLERR)
                    })
                });
            let writable = connected
                && fds
                    .get(1)
                    .is_some_and(|f| f.revents().is_some_and(|r| r.contains(PollFlags::POLLOUT)));
            let snapshot_ready = self.snapshots.is_some()
                && fds.get(snapshot_index).is_some_and(|fd| {
                    fd.revents().is_some_and(|flags| {
                        flags.intersects(PollFlags::POLLIN | PollFlags::POLLHUP)
                    })
                });
            drop(fds);
            if snapshot_ready {
                self.read_snapshots();
            }
            if writable {
                self.flush_reload();
            }
            if socket_ready {
                self.read_events(&mut line, &mut connected)?;
            }
            if let Some(log) = &mut self.log {
                log.maintain();
            }
            while let Some(signal) = signals.read_signal()? {
                match Signal::try_from(signal.ssi_signo as i32) {
                    Ok(Signal::SIGHUP) => self.reload(),
                    Ok(Signal::SIGTERM) => tracing::info!(
                        "termination requested; waiting for authenticated curtain exit"
                    ),
                    _ => {}
                }
            }
        }
    }

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
