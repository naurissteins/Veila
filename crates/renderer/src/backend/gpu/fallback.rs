use std::{
    any::Any,
    panic::{AssertUnwindSafe, catch_unwind},
};

use crate::Result;

pub(super) fn catch_gpu_panic(operation: impl FnOnce() -> Result<()>) -> Result<()> {
    catch_unwind(AssertUnwindSafe(operation))
        .map_err(|panic| crate::RendererError::FrameBackendUnavailable(panic_message(&panic)))?
}

fn panic_message(panic: &Box<dyn Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&'static str>() {
        return format!("gpu backend panicked: {message}");
    }
    if let Some(message) = panic.downcast_ref::<String>() {
        return format!("gpu backend panicked: {message}");
    }

    "gpu backend panicked".to_string()
}
