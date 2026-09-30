use smithay_client_toolkit::{output::OutputData, reexports::client::Proxy};

use super::ManagedLockSurface;

impl ManagedLockSurface {
    pub(crate) fn with_output_name<T>(&self, read: impl FnOnce(Option<&str>) -> T) -> T {
        match self.output.data::<OutputData>() {
            // SCTK publishes completed updates here; keep callbacks free of nested metadata reads.
            Some(data) => data.with_output_info(|info| read(info.name.as_deref())),
            None => read(None),
        }
    }
}
