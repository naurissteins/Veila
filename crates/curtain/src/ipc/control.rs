use calloop::ping::Ping;
use std::sync::mpsc::Sender;
use veila_common::{
    FingerprintStatus, NowPlayingSnapshot,
    ipc::{CurtainControlMessage, LockPowerStatusSnapshot},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlEvent {
    Supervisor(Box<veila_common::session::SupervisorMessage>),
    Unlock {
        attempt_id: Option<u64>,
    },
    ArmResumeInputGuard,
    MarkResumed,
    UpdateNowPlaying {
        snapshot: Option<NowPlayingSnapshot>,
    },
    UpdatePowerStatus {
        snapshot: Option<LockPowerStatusSnapshot>,
    },
    UpdateFingerprintStatus {
        status: Option<FingerprintStatus>,
    },
}

#[derive(Clone)]
pub(crate) struct ControlSender {
    sender: Sender<ControlEvent>,
    ping: Ping,
}

impl ControlSender {
    pub(crate) fn new(sender: Sender<ControlEvent>, ping: Ping) -> Self {
        Self { sender, ping }
    }

    pub(crate) fn send(
        &self,
        event: ControlEvent,
    ) -> std::result::Result<(), std::sync::mpsc::SendError<ControlEvent>> {
        self.sender.send(event)?;
        self.ping.ping();
        Ok(())
    }
}

pub(crate) fn control_event(message: CurtainControlMessage) -> ControlEvent {
    match message {
        CurtainControlMessage::Unlock { attempt_id } => ControlEvent::Unlock { attempt_id },
        CurtainControlMessage::ArmResumeInputGuard => ControlEvent::ArmResumeInputGuard,
        CurtainControlMessage::MarkResumed => ControlEvent::MarkResumed,
        CurtainControlMessage::UpdateNowPlaying { snapshot } => {
            ControlEvent::UpdateNowPlaying { snapshot }
        }
        CurtainControlMessage::UpdatePowerStatus { snapshot } => {
            ControlEvent::UpdatePowerStatus { snapshot }
        }
        CurtainControlMessage::UpdateFingerprintStatus { status } => {
            ControlEvent::UpdateFingerprintStatus { status }
        }
    }
}
