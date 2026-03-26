use smithay_client_toolkit::{
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shm::{Shm, ShmHandler},
};

use crate::state::CurtainApp;

impl ProvidesRegistryState for CurtainApp {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![
        smithay_client_toolkit::output::OutputState,
        smithay_client_toolkit::seat::SeatState
    ];
}

impl ShmHandler for CurtainApp {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}
