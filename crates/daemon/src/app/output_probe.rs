use anyhow::{Context, Result};
use smithay_client_toolkit::{
    output::{OutputHandler, OutputInfo, OutputState},
    reexports::client::{
        Connection, QueueHandle, globals::registry_queue_init, protocol::wl_output,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use veila_renderer::FrameSize;

pub(crate) fn current_outputs() -> Result<Vec<ProbedOutput>> {
    let connection =
        Connection::connect_to_env().context("failed to connect to Wayland for output probe")?;
    let (globals, mut event_queue) = registry_queue_init(&connection)
        .context("failed to enumerate Wayland globals for output probe")?;
    let queue_handle = event_queue.handle();
    let mut probe = OutputProbe {
        output_state: OutputState::new(&globals, &queue_handle),
        registry_state: RegistryState::new(&globals),
    };

    event_queue
        .roundtrip(&mut probe)
        .context("failed to complete initial output probe roundtrip")?;
    event_queue
        .roundtrip(&mut probe)
        .context("failed to complete output probe metadata roundtrip")?;

    let mut outputs = Vec::new();
    for output in probe.output_state.outputs() {
        if let Some(info) = probe.output_state.info(&output)
            && let Some(size) = logical_size(&info)
        {
            let scale = info.scale_factor.max(1);
            outputs.push(ProbedOutput {
                name: info.name.clone(),
                size: scaled_size(size, scale),
                scale,
            });
        }
    }

    Ok(outputs)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProbedOutput {
    pub(crate) name: Option<String>,
    pub(crate) size: FrameSize,
    pub(crate) scale: i32,
}

struct OutputProbe {
    output_state: OutputState,
    registry_state: RegistryState,
}

impl OutputHandler for OutputProbe {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ProvidesRegistryState for OutputProbe {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

fn logical_size(info: &OutputInfo) -> Option<FrameSize> {
    let (width, height) = info.logical_size?;
    if width > 0 && height > 0 {
        Some(FrameSize::new(width as u32, height as u32))
    } else {
        None
    }
}

fn scaled_size(size: FrameSize, scale: i32) -> FrameSize {
    let scale = scale.max(1) as u32;
    FrameSize::new(
        size.width.saturating_mul(scale),
        size.height.saturating_mul(scale),
    )
}

smithay_client_toolkit::delegate_output!(OutputProbe);
smithay_client_toolkit::delegate_registry!(OutputProbe);

#[cfg(test)]
mod tests {
    use super::scaled_size;
    use veila_renderer::FrameSize;

    #[test]
    fn output_probe_uses_physical_buffer_size_for_scaled_outputs() {
        assert_eq!(
            scaled_size(FrameSize::new(1920, 1080), 2),
            FrameSize::new(3840, 2160)
        );
    }

    #[test]
    fn output_probe_clamps_invalid_scale_to_one() {
        assert_eq!(
            scaled_size(FrameSize::new(1920, 1080), 0),
            FrameSize::new(1920, 1080)
        );
    }
}
