use super::Running;
use crate::integrations::{Event, Handle, Request, State};
use std::{io::Read, time::Instant};
use veila_common::{
    AppConfig,
    session::{self, SupervisorMessage, frame},
};

impl Running {
    pub(super) fn start_integrations(&mut self) {
        self.activity = Instant::now();
        if self.integrations.is_some() {
            self.publish_integrations();
            return;
        }
        let state = State {
            config: self.config.clone(),
            generation: self.generation,
            revision: self.revision,
            confirmed: self.locked,
            auth: self.auth_in_flight,
            activity: self.activity,
        };
        match Handle::start(state, self.options.config.clone()) {
            Ok(handle) => self.integrations = Some(handle),
            Err(error) => {
                tracing::warn!(%error, "integrations unavailable; local PAM remains available")
            }
        }
    }
    pub(super) fn publish_integrations(&self) {
        if let Some(handle) = &self.integrations {
            handle.state.send_modify(|state| {
                state.generation = self.generation;
                state.revision = self.revision;
                state.confirmed = self.locked;
                state.auth = self.auth_in_flight;
                state.activity = self.activity;
                if state.config != self.config {
                    state.config = self.config.clone();
                }
            });
        }
    }
    pub(super) fn read_integrations(&mut self) {
        let mut closed = false;
        if let Some(handle) = &mut self.integrations {
            let mut bytes = [0; 256];
            loop {
                match handle.reader.read(&mut bytes) {
                    Ok(0) => {
                        closed = true;
                        break;
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        closed = true;
                        break;
                    }
                }
            }
        }
        if closed {
            self.integrations = None;
            tracing::warn!("integration worker disconnected; local PAM remains available");
            return;
        }
        loop {
            let event = self
                .integrations
                .as_mut()
                .and_then(|h| h.events.try_recv().ok());
            match event {
                Some(Event::FingerprintMatch {
                    generation,
                    revision,
                    attempt_id,
                    authorization,
                }) if self.locked
                    && self.config.fingerprint.enabled
                    && generation == self.generation
                    && revision == self.revision
                    && authorization.is_current()
                    && self
                        .child
                        .as_mut()
                        .is_some_and(|child| matches!(child.try_wait(), Ok(None))) =>
                {
                    tracing::info!(
                        generation,
                        attempt_id,
                        "fingerprint authorized current curtain unlock"
                    );
                    self.queue(SupervisorMessage::Control(
                        veila_common::ipc::CurtainControlMessage::Unlock {
                            attempt_id: Some(attempt_id),
                        },
                    ));
                }
                Some(Event::Message {
                    generation,
                    revision,
                    message,
                }) if self.locked
                    && generation == self.generation
                    && revision.is_none_or(|r| r == self.revision) =>
                {
                    if matches!(
                        &message,
                        SupervisorMessage::Control(
                            veila_common::ipc::CurtainControlMessage::UpdateNowPlaying {
                                snapshot: Some(_)
                            }
                        )
                    ) {
                        tracing::debug!(
                            elapsed_ms = veila_common::elapsed_ms(self.started),
                            "supervisor delivered live now-playing update"
                        );
                    }
                    if matches!(
                        &message,
                        SupervisorMessage::UpdateWeather { snapshot: Some(_) }
                    ) {
                        tracing::debug!("supervisor delivered live weather update");
                    }
                    self.queue(message);
                }
                Some(Event::Reload {
                    generation,
                    revision,
                    config,
                }) if generation == self.generation && revision == self.revision => {
                    self.apply_reload(*config);
                }
                Some(Event::Arm { generation, ack })
                    if self.locked && generation == self.generation =>
                {
                    self.guard_request = self.guard_request.saturating_add(1);
                    if self.queue(SupervisorMessage::ArmResumeInputGuard {
                        request: self.guard_request,
                    }) {
                        self.guard_ack = Some((self.guard_request, ack));
                    }
                }
                Some(_) => tracing::debug!("discarding inactive generation integration event"),
                None => break,
            }
        }
    }
    pub(super) fn queue(&mut self, message: SupervisorMessage) -> bool {
        let bytes = match frame::encode(&message) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!(%error, "supervisor update rejected");
                return false;
            }
        };
        let reserve = if matches!(
            message,
            SupervisorMessage::Control(_) | SupervisorMessage::ArmResumeInputGuard { .. }
        ) {
            4096
        } else {
            0
        };
        if self.pending.len() + bytes.len() > session::MAX_FRAME_BYTES * 2 + reserve {
            tracing::warn!("supervisor update queue full; retaining current scene");
            return false;
        }
        self.pending.extend(bytes);
        self.flush_updates();
        true
    }
    pub(super) fn apply_reload(&mut self, config: AppConfig) {
        if !self.pending.is_empty() {
            tracing::warn!("update still pending; retaining current config");
            return;
        }
        if self.queue(SupervisorMessage::ReloadConfig {
            config: Box::new(config.clone()),
        }) {
            self.config = config;
            self.revision = self.revision.saturating_add(1);
            self.publish_integrations();
        }
    }
    pub(super) fn request_power(&self, action: veila_common::PowerAction) {
        if !self.locked || !self.config.visuals.power_button_enabled(action) {
            tracing::warn!(?action, "power request rejected by current config");
            return;
        }
        if let Some(handle) = &self.integrations {
            let _ = handle.requests.try_send(Request::Power {
                generation: self.generation,
                action,
            });
        }
    }
}

#[cfg(test)]
mod tests;
