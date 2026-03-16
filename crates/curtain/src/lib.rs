#![forbid(unsafe_code)]

//! Curtain crate bootstrap helpers.

/// Returns the component identifier used by logs and process supervision.
pub const fn component_name() -> &'static str {
    "kwylock-curtain"
}
