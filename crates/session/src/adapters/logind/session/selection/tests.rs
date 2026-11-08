use super::{
    SessionCandidate, SessionRejection, SessionSnapshot, select_session, session_selection_rank,
    validate_session,
};

fn desktop() -> SessionSnapshot {
    SessionSnapshot {
        uid: 1000,
        active: true,
        class: "user".into(),
        remote: false,
        state: "active".into(),
        session_type: "wayland".into(),
        seat: "seat0".into(),
    }
}

fn candidate(id: &str, snapshot: SessionSnapshot) -> SessionCandidate {
    SessionCandidate {
        id: id.into(),
        path: format!("/org/freedesktop/login1/session/{id}")
            .try_into()
            .expect("session path"),
        snapshot,
    }
}

#[test]
fn accepts_interactive_classes_and_local_tty_compositors() {
    for class in ["user", "user-early", "user-light", "user-early-light"] {
        for session_type in ["wayland", "tty"] {
            let snapshot = SessionSnapshot {
                class: class.into(),
                session_type: session_type.into(),
                ..desktop()
            };
            assert_eq!(validate_session(&snapshot, 1000), Ok(()));
        }
    }
}

#[test]
fn rejects_other_users_even_for_explicit_looking_desktop_sessions() {
    assert_eq!(
        validate_session(&desktop(), 1001),
        Err(SessionRejection::OtherUser)
    );
}

#[test]
fn rejects_manager_background_greeter_and_incomplete_sessions() {
    for class in [
        "manager",
        "manager-early",
        "background",
        "background-light",
        "greeter",
        "lock-screen",
        "user-incomplete",
        "",
    ] {
        let snapshot = SessionSnapshot {
            class: class.into(),
            ..desktop()
        };
        assert_eq!(
            validate_session(&snapshot, 1000),
            Err(SessionRejection::NonInteractive)
        );
    }
}

#[test]
fn rejects_remote_sessions() {
    let snapshot = SessionSnapshot {
        remote: true,
        ..desktop()
    };
    assert_eq!(
        validate_session(&snapshot, 1000),
        Err(SessionRejection::Remote)
    );
}

#[test]
fn rejects_other_session_types() {
    for session_type in ["x11", "mir", "web", "unspecified", ""] {
        let snapshot = SessionSnapshot {
            session_type: session_type.into(),
            ..desktop()
        };
        assert_eq!(
            validate_session(&snapshot, 1000),
            Err(SessionRejection::UnsupportedType)
        );
    }
}

#[test]
fn rejects_closing_or_unknown_states() {
    for state in ["closing", "opening", ""] {
        let snapshot = SessionSnapshot {
            state: state.into(),
            ..desktop()
        };
        assert_eq!(
            validate_session(&snapshot, 1000),
            Err(SessionRejection::Unavailable)
        );
    }
}

#[test]
fn accepts_inactive_online_sessions_for_vt_switch_and_explicit_selection() {
    let snapshot = SessionSnapshot {
        active: false,
        state: "online".into(),
        ..desktop()
    };
    assert_eq!(validate_session(&snapshot, 1000), Ok(()));
}

#[test]
fn lone_manager_session_does_not_bypass_validation() {
    let manager = SessionSnapshot {
        class: "manager".into(),
        session_type: "unspecified".into(),
        ..desktop()
    };
    assert!(select_session(vec![candidate("manager", manager)], 1000).is_err());
}

#[test]
fn rejects_empty_and_all_ineligible_fallbacks() {
    assert!(select_session(Vec::new(), 1000).is_err());
    let remote = SessionSnapshot {
        remote: true,
        ..desktop()
    };
    assert!(
        select_session(
            vec![candidate("remote", remote), candidate("other", desktop())],
            1001
        )
        .is_err()
    );
}

#[test]
fn eligible_desktop_wins_over_manager_and_remote_candidates() {
    let manager = SessionSnapshot {
        class: "manager".into(),
        ..desktop()
    };
    let remote = SessionSnapshot {
        remote: true,
        ..desktop()
    };
    let selected = select_session(
        vec![
            candidate("manager", manager),
            candidate("remote", remote),
            candidate("desktop", desktop()),
        ],
        1000,
    )
    .expect("desktop");
    assert_eq!(selected.id, "desktop");
}

#[test]
fn inactive_wayland_is_preferred_to_active_tty_during_uid_fallback() {
    let wayland = SessionSnapshot {
        active: false,
        state: "online".into(),
        seat: String::new(),
        ..desktop()
    };
    let tty = SessionSnapshot {
        session_type: "tty".into(),
        ..desktop()
    };
    assert!(session_selection_rank(&wayland) > session_selection_rank(&tty));
}

#[test]
fn equally_ranked_fallbacks_fail_regardless_of_list_order() {
    for ids in [["one", "two"], ["two", "one"]] {
        let error = select_session(
            ids.into_iter().map(|id| candidate(id, desktop())).collect(),
            1000,
        )
        .err()
        .expect("ambiguous");
        assert!(error.to_string().contains("ambiguous"));
    }
}

#[test]
fn lower_ranked_tie_does_not_block_a_unique_better_candidate() {
    let weaker = SessionSnapshot {
        active: false,
        state: "online".into(),
        ..desktop()
    };
    let selected = select_session(
        vec![
            candidate("weak1", weaker.clone()),
            candidate("weak2", weaker),
            candidate("best", desktop()),
        ],
        1000,
    )
    .expect("best");
    assert_eq!(selected.id, "best");
}
