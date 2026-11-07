use super::*;
use crate::{
    LockOptions,
    record::{LockRecord, Record, State as RecordState},
};
use std::{fs::File, os::unix::net::UnixStream, time::Duration};
use tokio::sync::{mpsc, watch};
use veila_common::{
    ipc::{CurtainControlMessage, LockLatencyReport},
    session::frame,
};

struct Harness {
    running: Running,
    curtain: UnixStream,
    events: mpsc::Sender<Event>,
    _wake: UnixStream,
    requests: mpsc::Receiver<Request>,
    path: std::path::PathBuf,
}
impl Harness {
    fn new() -> Self {
        let (channel, curtain) = UnixStream::pair().unwrap();
        channel.set_nonblocking(true).unwrap();
        curtain
            .set_read_timeout(Some(Duration::from_millis(30)))
            .unwrap();
        let (reader, wake) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        let (events, receiver) = mpsc::channel(4);
        let config = AppConfig::default();
        let (state, _) = watch::channel(State {
            config: config.clone(),
            generation: 2,
            revision: 0,
            confirmed: true,
            auth: false,
            activity: Instant::now(),
        });
        let (requests, request_rx) = mpsc::channel(4);
        let path = std::env::temp_dir().join(format!(
            "veila-integration-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let file = File::create(&path).unwrap();
        let record = LockRecord {
            file,
            recovered: false,
            value: Record {
                supervisor_pid: std::process::id(),
                supervisor_start_ticks: 0,
                curtain_pid: None,
                curtain_start_ticks: None,
                state: RecordState::Locked,
                config: None,
            },
        };
        Self {
            running: Running {
                log: None,
                options: LockOptions::default(),
                record,
                child: None,
                channel: Some(channel),
                recovery: Default::default(),
                notifications: Vec::new(),
                idle_notify: None,
                integrations: Some(Handle {
                    reader,
                    events: receiver,
                    state,
                    requests,
                }),
                generation: 2,
                revision: 0,
                auth_in_flight: false,
                activity: Instant::now(),
                guard_request: 0,
                guard_ack: None,
                pending: Vec::new(),
                written: 0,
                notified: true,
                locked: true,
                ready: true,
                started: Instant::now(),
                spawned: Instant::now(),
                report: LockLatencyReport::default(),
                config,
            },
            curtain,
            events,
            _wake: wake,
            requests: request_rx,
            path,
        }
    }
    fn event(&mut self, generation: u64) {
        self.events
            .try_send(Event::Message {
                generation,
                revision: None,
                message: SupervisorMessage::Control(CurtainControlMessage::Unlock {
                    attempt_id: None,
                }),
            })
            .unwrap();
        self.running.read_integrations();
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn stale_unlock_cannot_cross_a_replacement_generation() {
    let mut h = Harness::new();
    h.event(1);
    assert!(
        frame::read::<SupervisorMessage>(&mut std::io::BufReader::new(&mut h.curtain)).is_err()
    );
    h.event(2);
    assert_eq!(
        frame::read::<SupervisorMessage>(&mut std::io::BufReader::new(&mut h.curtain)).unwrap(),
        Some(SupervisorMessage::Control(CurtainControlMessage::Unlock {
            attempt_id: None
        }))
    );
}
#[test]
fn unconfirmed_generation_never_receives_an_integration_unlock() {
    let mut h = Harness::new();
    h.running.locked = false;
    h.event(2);
    assert!(h.running.pending.is_empty());
    assert!(
        frame::read::<SupervisorMessage>(&mut std::io::BufReader::new(&mut h.curtain)).is_err()
    );
}
#[test]
fn stale_guard_request_cannot_acknowledge_a_new_curtain() {
    let mut h = Harness::new();
    let (ack, mut response) = tokio::sync::oneshot::channel();
    h.events
        .try_send(Event::Arm { generation: 1, ack })
        .unwrap();
    h.running.read_integrations();
    assert!(h.running.guard_ack.is_none());
    assert!(matches!(
        response.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Closed)
    ));
}
#[test]
fn current_guard_request_waits_for_the_matching_curtain_acknowledgement() {
    let mut h = Harness::new();
    let (ack, mut response) = tokio::sync::oneshot::channel();
    h.events
        .try_send(Event::Arm { generation: 2, ack })
        .unwrap();
    h.running.read_integrations();
    let token = h.running.guard_request;
    h.running
        .event(veila_common::session::CurtainMessage::ResumeGuardArmed { request: token + 1 })
        .unwrap();
    assert!(matches!(
        response.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    h.running
        .event(veila_common::session::CurtainMessage::ResumeGuardArmed { request: token })
        .unwrap();
    assert!(response.try_recv().is_ok());
}

#[test]
fn power_requests_use_current_config_and_current_generation() {
    let mut h = Harness::new();
    h.running.request_power(veila_common::PowerAction::Reboot);
    assert!(h.requests.try_recv().is_err());
    h.running.config = AppConfig::from_toml_str("[visuals.power.reboot]\nenabled=true\n").unwrap();
    h.running.request_power(veila_common::PowerAction::Reboot);
    assert!(matches!(
        h.requests.try_recv(),
        Ok(Request::Power {
            generation: 2,
            action: veila_common::PowerAction::Reboot
        })
    ));
    h.running.config = AppConfig::default();
    h.running.request_power(veila_common::PowerAction::Reboot);
    assert!(h.requests.try_recv().is_err());
}

#[test]
fn old_config_revision_cannot_overwrite_current_widget_data() {
    let mut h = Harness::new();
    h.running.revision = 2;
    h.events
        .try_send(Event::Message {
            generation: 2,
            revision: Some(1),
            message: SupervisorMessage::UpdateWeather { snapshot: None },
        })
        .unwrap();
    h.running.read_integrations();
    assert!(
        frame::read::<SupervisorMessage>(&mut std::io::BufReader::new(&mut h.curtain)).is_err()
    );
}
