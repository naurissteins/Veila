use super::session::{SessionLookupCandidate, normalized_session_id, session_lookup_candidates};

#[test]
fn normalizes_session_ids() {
    assert_eq!(normalized_session_id(Some(" c2 ")).as_deref(), Some("c2"));
    assert_eq!(normalized_session_id(Some("   ")), None);
}

#[test]
fn explicit_session_id_has_no_fallback() {
    assert_eq!(
        session_lookup_candidates(Some(" c2 "), Some("manager")),
        vec![SessionLookupCandidate::Explicit("c2".into())]
    );
}

#[test]
fn implicit_lookup_keeps_environment_pid_and_uid_order() {
    assert_eq!(
        session_lookup_candidates(None, Some(" c2 ")),
        vec![
            SessionLookupCandidate::Environment("c2".into()),
            SessionLookupCandidate::Pid,
            SessionLookupCandidate::ListByUid
        ]
    );
}

#[test]
fn blank_session_ids_do_not_disable_automatic_resolution() {
    assert_eq!(
        session_lookup_candidates(Some(" "), Some(" ")),
        vec![
            SessionLookupCandidate::Pid,
            SessionLookupCandidate::ListByUid
        ]
    );
}
