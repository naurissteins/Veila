use super::*;

#[test]
fn component_filters_match_the_configured_lock_file_modules() {
    let curtain = "INFO veila_curtain::app: ready";
    let ui = "DEBUG veila_ui::shell: prompt updated";
    let supervisor = "INFO veila_session::app: confirmed";
    assert!(matches_target(curtain, LogTarget::Curtain));
    assert!(!matches_target(ui, LogTarget::Curtain));
    assert!(matches_target(ui, LogTarget::Ui));
    assert!(!matches_target(supervisor, LogTarget::Ui));
    assert!(matches_target(supervisor, LogTarget::All));
    assert!(!matches_target(curtain, LogTarget::Idle));
}
