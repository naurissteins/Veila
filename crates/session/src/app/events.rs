use super::Running;
use crate::record::State;
use anyhow::Result;
use std::io::Write;
use veila_common::{
    AppConfig, elapsed_ms, elapsed_us,
    session::{CurtainMessage, SupervisorMessage, frame},
};

impl Running {
    pub(super) fn event(&mut self, event: CurtainMessage) -> Result<()> {
        match event {
            CurtainMessage::SessionLocked if !self.locked => {
                if let Some(file) = &mut self.idle_notify {
                    let _ = frame::write(
                        file,
                        &veila_common::ipc::CurtainStartupMessage::SessionLocked,
                    );
                }
                self.recovery.confirmed = true;
                self.snapshots = if self.options.emergency {
                    None
                } else {
                    match crate::snapshots::start(self.config.clone()) {
                        Ok(reader) => Some(reader),
                        Err(error) => {
                            tracing::warn!(%error,"optional initial data unavailable");
                            None
                        }
                    }
                };
                self.locked = true;
                self.record.value.state = State::Locked;
                if let Err(error) = self.record.save() {
                    tracing::error!(%error, "failed to update confirmed lock record; retaining ownership");
                }
                tracing::info!("supervisor observed secure lock confirmation");
                if !self.options.wait_ready && !self.notified {
                    self.update_report();
                    self.notify(0, false)?;
                }
            }
            CurtainMessage::Ready { latency_report } if self.locked && !self.ready => {
                if let Some(file) = &mut self.idle_notify {
                    let _ = frame::write(
                        file,
                        &veila_common::ipc::CurtainStartupMessage::Ready {
                            latency_report: latency_report.clone(),
                        },
                    );
                }
                self.ready = true;
                self.report.curtain = latency_report.map(|report| *report);
                self.update_report();
                if !self.notified {
                    self.notify(0, false)?;
                }
            }
            CurtainMessage::Activity | CurtainMessage::AuthInFlight(_) => {}
            CurtainMessage::RequestPowerAction { action } => {
                tracing::warn!(?action, "supervisor power integration is not enabled yet")
            }
            _ => tracing::warn!("ignoring duplicate or out-of-order curtain startup event"),
        }
        Ok(())
    }
    fn update_report(&mut self) {
        self.report.activation_total_ms = elapsed_ms(self.started);
        self.report.activation_total_us = elapsed_us(self.started);
        self.report.curtain_ready_wait_ms = elapsed_ms(self.spawned);
        self.report.curtain_ready_wait_us = elapsed_us(self.spawned);
    }
    pub(super) fn reload(&mut self) {
        let config = match AppConfig::load(self.options.config.as_deref()) {
            Ok(loaded) => loaded.config,
            Err(error) => {
                tracing::warn!(%error,"reload rejected; retaining current config");
                return;
            }
        };
        let payload = match frame::encode(&SupervisorMessage::ReloadConfig {
            config: Box::new(config.clone()),
        }) {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!(%error,"resolved reload exceeds channel limit");
                return;
            }
        };
        if !self.pending.is_empty() {
            tracing::warn!("reload still pending; retaining current config");
            return;
        }
        self.snapshots = None;
        self.pending = payload;
        self.written = 0;
        self.config = config;
        self.flush_reload();
    }
    pub(super) fn flush_reload(&mut self) {
        let Some(channel) = &mut self.channel else {
            return;
        };
        match channel.write(&self.pending[self.written..]) {
            Ok(0) => {
                self.pending.clear();
                self.written = 0;
            }
            Ok(len) => {
                self.written += len;
                if self.written == self.pending.len() {
                    self.pending.clear();
                    self.written = 0;
                    tracing::info!("sent resolved config reload");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(error) => {
                tracing::warn!(%error,"reload channel unavailable");
                self.pending.clear();
                self.written = 0;
            }
        }
    }
}
