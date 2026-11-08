#![forbid(unsafe_code)]

mod client;
mod helper;
pub mod policy;
pub mod protocol;

pub use client::{Challenge, ClientError, Conversation, Verdict, authenticate};
pub use helper::{
    HelperError, PAM_HELPER_PROCESS_NAME, PAM_HELPER_SUBCOMMAND, helper_command,
    report_service_selection, run_helper,
    service::{PamService, selected_service},
};
