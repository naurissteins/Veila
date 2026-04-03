use super::session::{
    SessionLookupCandidate, SessionSelectionSnapshot, normalized_session_id,
    session_lookup_candidates, session_selection_score,
};

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

#[test]
fn prefers_active_local_user_session_for_uid_fallback() {
    let preferred = SessionSelectionSnapshot {
        active: true,
        class: "user".to_string(),
        remote: false,
        state: "active".to_string(),
        session_type: "wayland".to_string(),
        seat: "seat0".to_string(),
    };
    let weaker = SessionSelectionSnapshot {
        active: false,
        class: "manager".to_string(),
        remote: false,
        state: "online".to_string(),
        session_type: "".to_string(),
        seat: "".to_string(),
    };

    assert!(
        session_selection_score(&preferred, Some("wayland"))
            > session_selection_score(&weaker, Some("wayland"))
    );
}
