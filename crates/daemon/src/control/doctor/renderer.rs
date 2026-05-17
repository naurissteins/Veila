use std::{collections::HashSet, path::Path};

use anyhow::{Context, Result};
use smithay_client_toolkit::reexports::client::{
    Connection, Dispatch, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry,
};
use veila_common::{AppConfig, OutputUiMode};
use veila_renderer::FrameSize;

use crate::app::output_probe::{self, ProbedOutput};

use super::{CheckStatus, DoctorSummary};

const MAX_SHM_SLOTS_PER_OUTPUT: u64 = 2;
const HIGH_POST_READY_KIB: u64 = 512 * 1024;

pub(super) fn check_renderer(summary: &mut DoctorSummary, config_path: Option<&Path>) {
    let config = match AppConfig::load(config_path) {
        Ok(loaded) => loaded.config,
        Err(error) => {
            println!("renderer.outputs=unknown");
            summary.record(
                "renderer",
                CheckStatus::Warning,
                format!("renderer diagnostics skipped because config failed to load: {error}"),
            );
            return;
        }
    };

    let protocols = match probe_renderer_protocols() {
        Ok(protocols) => protocols,
        Err(error) => {
            println!("renderer.protocols=unknown");
            summary.record(
                "renderer",
                CheckStatus::Warning,
                format!("failed to probe renderer Wayland protocols: {error}"),
            );
            return;
        }
    };

    println!("renderer.wl_shm={}", protocols.wl_shm);
    println!("renderer.wp_viewporter={}", protocols.wp_viewporter);
    println!(
        "renderer.wp_fractional_scale_manager_v1={}",
        protocols.wp_fractional_scale_manager_v1
    );

    let outputs = match output_probe::current_outputs() {
        Ok(outputs) => outputs,
        Err(error) => {
            println!("renderer.outputs=unknown");
            summary.record(
                "renderer",
                CheckStatus::Warning,
                format!("failed to probe output sizes for renderer diagnostics: {error}"),
            );
            return;
        }
    };

    let ui_mode = config.visuals.output_ui_mode();
    let ui_output = config.visuals.ui_output_name();
    let visible_outputs = visible_output_count(&outputs, ui_mode);
    let scene_base_kib = estimate_scene_base_kib(&outputs, ui_mode, ui_output);
    let one_slot_shm_kib = estimate_shm_kib(&outputs, 1);
    let max_slot_shm_kib = estimate_shm_kib(&outputs, MAX_SHM_SLOTS_PER_OUTPUT);
    let ready_kib = scene_base_kib.saturating_add(one_slot_shm_kib);
    let post_ready_kib = scene_base_kib.saturating_add(max_slot_shm_kib);

    println!("renderer.outputs={}", outputs.len());
    println!("renderer.ui_mode={}", ui_mode_label(ui_mode));
    println!("renderer.ui_output={}", ui_output.unwrap_or("auto"));
    println!("renderer.ui_visible_outputs={visible_outputs}");
    for (index, output) in outputs.iter().enumerate() {
        println!(
            "renderer.output.{index}.name={}",
            output.name.as_deref().unwrap_or("unknown")
        );
        println!("renderer.output.{index}.scale={}", output.scale);
        println!("renderer.output.{index}.buffer_width={}", output.size.width);
        println!(
            "renderer.output.{index}.buffer_height={}",
            output.size.height
        );
        println!(
            "renderer.output.{index}.frame_kib={}",
            frame_kib(output.size)
        );
    }
    println!("renderer.scene_base_estimated_kib={scene_base_kib}");
    println!("renderer.shm_one_slot_estimated_kib={one_slot_shm_kib}");
    println!("renderer.shm_two_slot_estimated_kib={max_slot_shm_kib}");
    println!("renderer.ready_estimated_kib={ready_kib}");
    println!("renderer.post_ready_estimated_kib={post_ready_kib}");

    if !protocols.wl_shm {
        summary.record(
            "renderer",
            CheckStatus::Error,
            "compositor does not advertise wl_shm",
        );
    } else if outputs.is_empty() {
        summary.record(
            "renderer",
            CheckStatus::Error,
            "no output sizes were available for renderer diagnostics",
        );
    } else if post_ready_kib > HIGH_POST_READY_KIB {
        summary.record(
            "renderer",
            CheckStatus::Warning,
            format!(
                "estimated post-ready renderer memory is high: {} MiB",
                post_ready_kib / 1024
            ),
        );
    } else {
        summary.record(
            "renderer",
            CheckStatus::Ok,
            "renderer protocol support and memory estimates look normal",
        );
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct RendererProtocols {
    wl_shm: bool,
    wp_viewporter: bool,
    wp_fractional_scale_manager_v1: bool,
}

fn probe_renderer_protocols() -> Result<RendererProtocols> {
    let connection =
        Connection::connect_to_env().context("failed to connect to Wayland compositor")?;
    let (globals, _event_queue) = registry_queue_init::<RendererRegistryProbe>(&connection)
        .context("failed to enumerate Wayland globals")?;
    let globals = globals.contents().clone_list();

    Ok(RendererProtocols {
        wl_shm: globals.iter().any(|global| global.interface == "wl_shm"),
        wp_viewporter: globals
            .iter()
            .any(|global| global.interface == "wp_viewporter"),
        wp_fractional_scale_manager_v1: globals
            .iter()
            .any(|global| global.interface == "wp_fractional_scale_manager_v1"),
    })
}

fn visible_output_count(outputs: &[ProbedOutput], ui_mode: OutputUiMode) -> usize {
    match ui_mode {
        OutputUiMode::All => outputs.len(),
        OutputUiMode::Single => usize::from(!outputs.is_empty()),
    }
}

fn estimate_scene_base_kib(
    outputs: &[ProbedOutput],
    ui_mode: OutputUiMode,
    ui_output: Option<&str>,
) -> u64 {
    let visible = match ui_mode {
        OutputUiMode::All => outputs.iter().collect(),
        OutputUiMode::Single => select_single_output(outputs, ui_output)
            .into_iter()
            .collect(),
    };
    unique_frame_kib(visible)
}

fn select_single_output<'a>(
    outputs: &'a [ProbedOutput],
    ui_output: Option<&str>,
) -> Option<&'a ProbedOutput> {
    ui_output
        .and_then(|name| {
            outputs
                .iter()
                .find(|output| output.name.as_deref() == Some(name))
        })
        .or_else(|| outputs.first())
}

fn unique_frame_kib(outputs: Vec<&ProbedOutput>) -> u64 {
    let mut seen = HashSet::with_capacity(outputs.len());
    outputs
        .into_iter()
        .filter(|output| seen.insert((output.size.width, output.size.height)))
        .map(|output| frame_kib(output.size))
        .sum()
}

fn estimate_shm_kib(outputs: &[ProbedOutput], slots_per_output: u64) -> u64 {
    outputs
        .iter()
        .map(|output| frame_kib(output.size).saturating_mul(slots_per_output))
        .sum()
}

fn frame_kib(size: FrameSize) -> u64 {
    size.byte_len()
        .map(|byte_len| (byte_len / 1024) as u64)
        .unwrap_or(0)
}

const fn ui_mode_label(ui_mode: OutputUiMode) -> &'static str {
    match ui_mode {
        OutputUiMode::All => "all",
        OutputUiMode::Single => "single",
    }
}

struct RendererRegistryProbe;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for RendererRegistryProbe {
    fn event(
        _state: &mut Self,
        _registry: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use veila_renderer::FrameSize;

    use super::{
        ProbedOutput, estimate_scene_base_kib, estimate_shm_kib, frame_kib, visible_output_count,
    };
    use veila_common::OutputUiMode;

    fn output(name: &str, size: FrameSize) -> ProbedOutput {
        ProbedOutput {
            name: Some(name.to_string()),
            size,
            scale: 1,
        }
    }

    #[test]
    fn estimates_shm_for_every_output_slot() {
        let outputs = [
            output("DP-1", FrameSize::new(2560, 1440)),
            output("DP-2", FrameSize::new(5120, 2880)),
        ];

        assert_eq!(
            estimate_shm_kib(&outputs, 2),
            frame_kib(outputs[0].size) * 2 + frame_kib(outputs[1].size) * 2
        );
    }

    #[test]
    fn all_mode_deduplicates_matching_scene_base_sizes() {
        let outputs = [
            output("DP-1", FrameSize::new(2560, 1440)),
            output("DP-2", FrameSize::new(5120, 2880)),
            output("DP-3", FrameSize::new(5120, 2880)),
        ];

        assert_eq!(
            estimate_scene_base_kib(&outputs, OutputUiMode::All, None),
            frame_kib(outputs[0].size) + frame_kib(outputs[1].size)
        );
    }

    #[test]
    fn single_mode_estimates_one_visible_scene_base() {
        let outputs = [
            output("DP-1", FrameSize::new(2560, 1440)),
            output("DP-2", FrameSize::new(5120, 2880)),
        ];

        assert_eq!(visible_output_count(&outputs, OutputUiMode::Single), 1);
        assert_eq!(
            estimate_scene_base_kib(&outputs, OutputUiMode::Single, Some("DP-2")),
            frame_kib(outputs[1].size)
        );
    }
}
