use super::Running;
use anyhow::Result;
use std::io::Read;
use veila_common::{
    ipc::{LineAccumulator, LineProgress},
    session::{CurtainMessage, frame},
};
impl Running {
    pub(super) fn read_events(
        &mut self,
        line: &mut LineAccumulator,
        connected: &mut bool,
    ) -> Result<()> {
        loop {
            let mut bytes = [0; 8192];
            let Some(channel) = &mut self.channel else {
                break;
            };
            match channel.read(&mut bytes) {
                Ok(0) => {
                    *connected = false;
                    break;
                }
                Ok(len) => {
                    let mut pending = &bytes[..len];
                    while !pending.is_empty() {
                        match line.push_chunk(pending, "curtain event") {
                            Ok(LineProgress::Complete {
                                line: message,
                                consumed,
                            }) => {
                                match frame::decode::<CurtainMessage>(&message) {
                                    Ok(event) => self.event(event)?,
                                    Err(error) => {
                                        tracing::warn!(%error,"invalid curtain event");
                                        *connected = false;
                                        break;
                                    }
                                }
                                pending = &pending[consumed..];
                            }
                            Ok(LineProgress::Pending) => break,
                            Err(error) => {
                                tracing::warn!(%error,"invalid curtain framing");
                                *connected = false;
                                break;
                            }
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(error) => {
                    tracing::warn!(%error,"curtain channel failed; waiting for curtain exit");
                    *connected = false;
                    break;
                }
            }
            if !*connected {
                break;
            }
        }
        Ok(())
    }
}

impl Running {
    pub(super) fn read_snapshots(&mut self) {
        let mut bytes = [0; 4096];
        let Some(reader) = &mut self.snapshots else {
            return;
        };
        match reader.read(&mut bytes) {
            Ok(0) => self.snapshots = None,
            Ok(len) => {
                let mut pending = &bytes[..len];
                while !pending.is_empty() {
                    match self.snapshot_line.push_chunk(pending, "initial snapshot") {
                        Ok(LineProgress::Complete { line, consumed }) => {
                            let valid = matches!(
                                frame::decode::<veila_common::session::SupervisorMessage>(&line),
                                Ok(
                                    veila_common::session::SupervisorMessage::UpdateBattery { .. }
                                        | veila_common::session::SupervisorMessage::UpdateWeather { .. }
                                )
                            );
                            if valid
                                && self.pending.len() + line.len()
                                    < veila_common::session::MAX_FRAME_BYTES
                            {
                                self.pending.extend(line.as_bytes());
                                self.pending.push(b'\n');
                            }
                            pending = &pending[consumed..];
                        }
                        Ok(LineProgress::Pending) => break,
                        Err(_) => {
                            self.snapshots = None;
                            break;
                        }
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => self.snapshots = None,
        }
    }
}
