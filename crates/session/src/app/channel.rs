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
