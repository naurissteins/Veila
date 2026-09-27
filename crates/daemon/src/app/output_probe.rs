use anyhow::{Context, Result};
use smithay_client_toolkit::{
    output::{OutputHandler, OutputInfo, OutputState},
    reexports::client::{
        Connection, QueueHandle, globals::registry_queue_init, protocol::wl_output,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
};
use veila_renderer::{FrameSize, RenderScale};

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
            let render_scale = info
                .modes
                .iter()
                .find(|mode| mode.current)
                .and_then(|mode| RenderScale::infer_from_mode(size, mode.dimensions, scale))
                .unwrap_or_else(|| RenderScale::from_integer(scale as u32));
            outputs.push(ProbedOutput {
                name: info.name.clone(),
                size: render_scale.frame_size(size),
                scale,
                render_scale,
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
    pub(crate) render_scale: RenderScale,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_fractional_pixel_ratio_from_mode_and_logical_size() {
        let logical = FrameSize::new(1280, 720);
        assert_eq!(
            RenderScale::infer_from_mode(logical, (1920, 1080), 2)
                .unwrap()
                .units(),
            180
        );
        assert_eq!(
            RenderScale::infer_from_mode(logical, (1080, 1920), 2)
                .unwrap()
                .units(),
            180
        );
        assert_eq!(
            RenderScale::infer_from_mode(logical, (1600, 900), 2)
                .unwrap()
                .units(),
            150
        );
    }

    #[test]
    fn inconsistent_or_missing_mode_uses_integer_fallback() {
        let logical = FrameSize::new(1280, 720);
        assert!(RenderScale::infer_from_mode(logical, (1920, 900), 2).is_none());
        assert!(RenderScale::infer_from_mode(logical, (0, 1080), 2).is_none());
        assert!(RenderScale::infer_from_mode(logical, (1920, 1080), 1).is_none());
    }
}
