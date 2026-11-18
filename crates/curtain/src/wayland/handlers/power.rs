use smithay_client_toolkit::reexports::client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum, protocol::wl_output,
};
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1;

use crate::state::CurtainApp;

impl CurtainApp {
    pub(crate) fn restore_output_power_after_recovery(&mut self, qh: &QueueHandle<Self>) {
        if !std::mem::take(&mut self.restore_output_power_on_lock) {
            return;
        }
        let Ok(manager) = self.output_power_manager.get() else {
            return;
        };
        for surface in &self.lock_surfaces {
            if let Some(control) = &surface.output_power {
                control.set_mode(zwlr_output_power_v1::Mode::On);
            } else {
                let control = manager.get_output_power(&surface.output, qh, surface.output.clone());
                control.set_mode(zwlr_output_power_v1::Mode::On);
                // recovery-only controls must release the output to other power managers
                control.destroy();
            }
        }
    }
}

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
