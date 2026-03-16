#![forbid(unsafe_code)]

//! UI crate bootstrap helpers.

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "kwylock-ui"
}
