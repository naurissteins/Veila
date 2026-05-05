mod input;
mod power;
mod registry;
mod session;

use smithay_client_toolkit::reexports::client::protocol::wl_buffer;
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_manager_v1;

use crate::state::CurtainApp;

smithay_client_toolkit::delegate_compositor!(CurtainApp);
smithay_client_toolkit::delegate_keyboard!(CurtainApp);
smithay_client_toolkit::delegate_output!(CurtainApp);
smithay_client_toolkit::delegate_pointer!(CurtainApp);
smithay_client_toolkit::delegate_registry!(CurtainApp);
smithay_client_toolkit::delegate_seat!(CurtainApp);
smithay_client_toolkit::delegate_session_lock!(CurtainApp);
smithay_client_toolkit::delegate_shm!(CurtainApp);
smithay_client_toolkit::reexports::client::delegate_noop!(
    CurtainApp: ignore zwlr_output_power_manager_v1::ZwlrOutputPowerManagerV1
);
smithay_client_toolkit::reexports::client::delegate_noop!(CurtainApp: ignore wl_buffer::WlBuffer);
