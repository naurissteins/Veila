use super::Running;
use anyhow::Result;
use nix::{
    poll::{PollFd, PollFlags, PollTimeout, poll},
    sys::{signal::Signal, signalfd::SignalFd},
};
use std::{os::fd::AsFd, process::Child, time::Duration};
use veila_common::{ipc::LineAccumulator, session};

impl Running {
    pub(super) fn event_loop(&mut self, signals: SignalFd) -> Result<i32> {
        let mut line = LineAccumulator::with_limit(session::MAX_FRAME_BYTES);
        let mut connected = self.channel.is_some();
        loop {
            if self.recovery.due() {
                self.launch()?;
                line = LineAccumulator::with_limit(session::MAX_FRAME_BYTES);
                connected = self.channel.is_some();
            }
            if let Some(status) = self
                .child
                .as_mut()
                .map(Child::try_wait)
                .transpose()?
                .flatten()
            {
                if connected {
                    self.read_events(&mut line, &mut connected)?;
                }
                self.child = None;
                self.channel = None;
                connected = false;
                if let Some(code) = self.exited(status)? {
                    return Ok(code);
                }
                continue;
            }
            // readiness failure reports an error without stopping an already-secure, usable curtain
            let timeout = Duration::from_secs(self.config.lock.acquire_timeout_seconds.max(1))
                .saturating_add(Duration::from_secs(2));
            if !self.notified && self.spawned.elapsed() > timeout {
                self.notify(1, false)?;
            }
            let mut fds = vec![PollFd::new(signals.as_fd(), PollFlags::POLLIN)];
            if let Some(channel) = self.channel.as_ref().filter(|_| connected) {
                fds.push(PollFd::new(
                    channel.as_fd(),
                    PollFlags::POLLIN
                        | if self.pending.is_empty() {
                            PollFlags::empty()
                        } else {
                            PollFlags::POLLOUT
                        },
                ));
            }
            let integration_index = fds.len();
            if let Some(handle) = &self.integrations {
                fds.push(PollFd::new(handle.reader.as_fd(), PollFlags::POLLIN));
            }
            let wait = self.poll_timeout(timeout);
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
            let integration_ready = self.integrations.is_some()
                && fds.get(integration_index).is_some_and(|fd| {
                    fd.revents().is_some_and(|flags| {
                        flags.intersects(PollFlags::POLLIN | PollFlags::POLLHUP)
                    })
                });
            drop(fds);
            if integration_ready {
                self.read_integrations();
            }
            if writable {
                self.flush_updates();
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

    fn poll_timeout(&self, startup: Duration) -> PollTimeout {
        let mut wait = self.recovery.remaining();
        if !self.notified {
            let remaining = startup.saturating_sub(self.spawned.elapsed());
            wait = Some(wait.map_or(remaining, |delay| delay.min(remaining)));
        }
        if self.log.is_some() {
            wait = Some(wait.map_or(Duration::from_secs(15), |delay| {
                delay.min(Duration::from_secs(15))
            }));
        }
        wait.map_or(PollTimeout::NONE, |delay| {
            PollTimeout::try_from(delay).unwrap_or(PollTimeout::MAX)
        })
    }
}
