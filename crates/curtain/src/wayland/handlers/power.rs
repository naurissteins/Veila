use smithay_client_toolkit::reexports::client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum, protocol::wl_output,
};
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1;

use crate::state::CurtainApp;

impl Dispatch<zwlr_output_power_v1::ZwlrOutputPowerV1, wl_output::WlOutput> for CurtainApp {
    fn event(
        state: &mut Self,
        proxy: &zwlr_output_power_v1::ZwlrOutputPowerV1,
        event: zwlr_output_power_v1::Event,
        output: &wl_output::WlOutput,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_power_v1::Event::Mode { mode } => match mode {
                WEnum::Value(zwlr_output_power_v1::Mode::On) => {
                    tracing::debug!(
                        id = output.id().protocol_id(),
                        "locked output power mode is on"
                    );
                }
                WEnum::Value(zwlr_output_power_v1::Mode::Off) => {
                    tracing::debug!(
                        id = output.id().protocol_id(),
                        "locked output power mode is off"
                    );
                }
                WEnum::Value(_) => {
                    tracing::debug!(
                        id = output.id().protocol_id(),
                        "locked output reported unsupported power mode"
                    );
                }
                WEnum::Unknown(raw) => {
                    tracing::debug!(
                        id = output.id().protocol_id(),
                        raw,
                        "locked output reported unknown power mode"
                    );
                }
            },
            zwlr_output_power_v1::Event::Failed => {
                tracing::warn!(
                    id = output.id().protocol_id(),
                    "locked output power control became unavailable"
                );
                proxy.destroy();
                for surface in &mut state.lock_surfaces {
                    if surface.output == *output {
                        surface.output_power = None;
                    }
                }
            }
            _ => {}
        }
    }
}
