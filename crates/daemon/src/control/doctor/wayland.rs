use smithay_client_toolkit::reexports::client::{
    Connection, Dispatch, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::wl_registry,
};

use super::{CheckStatus, DoctorSummary};

pub(super) fn check_wayland(summary: &mut DoctorSummary) {
    let connection = match Connection::connect_to_env() {
        Ok(connection) => connection,
        Err(error) => {
            summary.record(
                "wayland",
                CheckStatus::Error,
                format!("failed to connect to Wayland compositor: {error}"),
            );
            return;
        }
    };

    let (globals, _event_queue) = match registry_queue_init::<WaylandRegistryProbe>(&connection) {
        Ok(result) => result,
        Err(error) => {
            summary.record(
                "wayland",
                CheckStatus::Error,
                format!("failed to enumerate Wayland globals: {error}"),
            );
            return;
        }
    };

    let globals = globals.contents().clone_list();
    let output_count = globals
        .iter()
        .filter(|global| global.interface == "wl_output")
        .count();
    let session_lock_version = globals
        .iter()
        .filter(|global| global.interface == "ext_session_lock_manager_v1")
        .map(|global| global.version)
        .max();

    println!("wayland.outputs={output_count}");
    println!(
        "wayland.ext_session_lock_manager_v1={}",
        session_lock_version
            .map(|version| version.to_string())
            .as_deref()
            .unwrap_or("missing")
    );

    match (session_lock_version, output_count) {
        (Some(_), 1..) => summary.record(
            "wayland",
            CheckStatus::Ok,
            "compositor advertises ext-session-lock and outputs",
        ),
        (None, _) => summary.record(
            "wayland",
            CheckStatus::Error,
            "compositor does not advertise ext-session-lock-v1",
        ),
        (Some(_), 0) => summary.record(
            "wayland",
            CheckStatus::Error,
            "compositor advertised no outputs",
        ),
    }
}

struct WaylandRegistryProbe;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for WaylandRegistryProbe {
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
