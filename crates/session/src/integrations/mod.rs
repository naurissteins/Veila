mod actions;
mod logind;
mod providers;
mod worker;

use std::{io::Write, os::unix::net::UnixStream, path::PathBuf, time::Instant};
use tokio::sync::{mpsc, oneshot, watch};
use veila_common::{AppConfig, PowerAction, session::SupervisorMessage};

#[derive(Clone)]
pub(crate) struct State {
    pub config: AppConfig,
    pub generation: u64,
    pub revision: u64,
    pub confirmed: bool,
    pub auth: bool,
    pub activity: Instant,
}

pub(crate) enum Event {
    Message {
        generation: u64,
        revision: Option<u64>,
        message: SupervisorMessage,
    },
    Reload {
        generation: u64,
        revision: u64,
        config: Box<AppConfig>,
    },
    Arm {
        generation: u64,
        ack: oneshot::Sender<()>,
    },
}

pub(crate) enum Request {
    Power {
        generation: u64,
        action: PowerAction,
    },
    Suspend {
        generation: u64,
    },
    Finish(std::sync::mpsc::Sender<()>),
}

pub(crate) struct Handle {
    pub reader: UnixStream,
    pub events: mpsc::Receiver<Event>,
    pub state: watch::Sender<State>,
    pub requests: mpsc::Sender<Request>,
}

impl Handle {
    pub fn start(state: State, path: Option<PathBuf>) -> std::io::Result<Self> {
        let (reader, wake) = UnixStream::pair()?;
        reader.set_nonblocking(true)?;
        wake.set_nonblocking(true)?;
        let (state, changes) = watch::channel(state);
        let (tx, events) = mpsc::channel(64);
        let (requests, rx) = mpsc::channel(16);
        let commands = requests.clone();
        std::thread::Builder::new()
            .name("veila-integrations".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                match runtime {
                    Ok(runtime) => {
                        runtime.block_on(worker::run(
                            changes,
                            rx,
                            Output { tx, wake },
                            path,
                            commands,
                        ));
                        runtime.shutdown_timeout(std::time::Duration::from_millis(50));
                    }
                    Err(error) => {
                        tracing::warn!(%error, "integrations unavailable; local PAM remains active")
                    }
                }
            })?;
        Ok(Self {
            reader,
            events,
            state,
            requests,
        })
    }

    pub fn finish(&self) {
        let (ack, done) = std::sync::mpsc::channel();
        if self.requests.try_send(Request::Finish(ack)).is_ok()
            && done
                .recv_timeout(std::time::Duration::from_secs(1))
                .is_err()
        {
            tracing::warn!("timed out clearing best-effort logind hint");
        }
    }
}

struct Output {
    tx: mpsc::Sender<Event>,
    wake: UnixStream,
}
impl Output {
    async fn send(&mut self, event: Event) -> bool {
        if self.tx.send(event).await.is_err() {
            return false;
        }
        // A full wake socket already makes the supervisor readable.
        let _ = self.wake.write(&[1]);
        true
    }
    async fn message(&mut self, generation: u64, message: SupervisorMessage) {
        self.send(Event::Message {
            generation,
            revision: None,
            message,
        })
        .await;
    }
}
