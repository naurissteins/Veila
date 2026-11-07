use super::{AuthEvent, BackgroundEvent};
use calloop::{channel::Channel, ping::PingSource};

pub(crate) struct CurtainEventSources {
    pub(crate) auth: Channel<AuthEvent>,
    pub(crate) background: Channel<BackgroundEvent>,
    pub(crate) control: PingSource,
}
