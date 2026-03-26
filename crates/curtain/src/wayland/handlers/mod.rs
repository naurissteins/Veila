mod input;
mod registry;
mod session;

use smithay_client_toolkit::reexports::client::protocol::wl_buffer;

use crate::state::CurtainApp;

smithay_client_toolkit::delegate_compositor!(CurtainApp);
smithay_client_toolkit::delegate_keyboard!(CurtainApp);
smithay_client_toolkit::delegate_output!(CurtainApp);
smithay_client_toolkit::delegate_pointer!(CurtainApp);
smithay_client_toolkit::delegate_registry!(CurtainApp);
smithay_client_toolkit::delegate_seat!(CurtainApp);
smithay_client_toolkit::delegate_session_lock!(CurtainApp);
smithay_client_toolkit::delegate_shm!(CurtainApp);
smithay_client_toolkit::reexports::client::delegate_noop!(CurtainApp: ignore wl_buffer::WlBuffer);
