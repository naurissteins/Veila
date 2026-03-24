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

pub(super) fn current_output_sizes() -> Result<Vec<FrameSize>> {
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

    let mut sizes = Vec::new();
    for output in probe.output_state.outputs() {
        if let Some(info) = probe.output_state.info(&output)
            && let Some(size) = logical_size(&info)
        {
            sizes.push(size);
        }
    }

    Ok(sizes)
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

smithay_client_toolkit::delegate_output!(OutputProbe);
smithay_client_toolkit::delegate_registry!(OutputProbe);
