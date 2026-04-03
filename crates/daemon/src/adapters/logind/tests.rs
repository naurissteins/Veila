use super::session::{SessionLookupCandidate, normalized_session_id, session_lookup_candidates};

#[test]
fn normalizes_session_ids() {
    assert_eq!(normalized_session_id(Some(" c2 ")).as_deref(), Some("c2"));
    assert_eq!(normalized_session_id(Some("   ")), None);
}

#[test]
fn prefers_cli_session_id_before_pid_lookup() {
    let candidates = session_lookup_candidates(Some("c2"));

    assert_eq!(
        candidates.first(),
        Some(&SessionLookupCandidate::SessionId {
            source: "cli",
            value: "c2".to_string(),
        })
    );
    assert!(candidates.contains(&SessionLookupCandidate::Pid));
    assert!(matches!(
        candidates.last(),
        Some(SessionLookupCandidate::ListByUid { .. })
    ));
}
