use super::{validate_invocation_mode, validate_preview_mode};
use crate::CurtainOptions;

#[test]
fn rejects_plain_no_argument_launches() {
    let options =
        CurtainOptions::parse_args(["veila-curtain".to_string()]).expect("arguments parse");
    let error =
        validate_invocation_mode(&options).expect_err("plain direct launch should be rejected");

    assert!(
        error
            .to_string()
            .contains("refusing to start a real lock session")
    );
}

#[test]
fn preview_mode_requires_png_and_rejects_lock_flags() {
    let parse = |args: &[&str]| {
        CurtainOptions::parse_args(
            std::iter::once("veila")
                .chain(args.iter().copied())
                .map(String::from),
        )
        .expect("arguments should parse")
    };

    assert!(validate_preview_mode(&parse(&["--preview-png=/tmp/p.png"])).is_ok());
    assert!(validate_preview_mode(&parse(&[])).is_err());
    assert!(validate_preview_mode(&parse(&["--preview-png=/tmp/p.png", "--lock"])).is_err());
}

#[test]
fn initial_backoff_belongs_only_to_direct_authentication() {
    let direct = CurtainOptions {
        lock: true,
        initial_backoff_ms: 500,
        ..Default::default()
    };
    assert!(validate_invocation_mode(&direct).is_ok());
    assert!(
        validate_preview_mode(&CurtainOptions {
            preview_png: Some("/tmp/a.png".into()),
            lock: false,
            ..direct
        })
        .is_err()
    );
}

#[test]
fn inherited_session_requires_complete_distinct_descriptors_and_local_pam() {
    let base = CurtainOptions {
        lock: true,
        ipc_fd: Some(3),
        lock_fd: Some(4),
        ..Default::default()
    };
    assert!(validate_invocation_mode(&base).is_ok());
    for options in [
        CurtainOptions {
            ipc_fd: None,
            ..base.clone()
        },
        CurtainOptions {
            lock_fd: None,
            ..base.clone()
        },
        CurtainOptions {
            lock_fd: Some(3),
            ..base.clone()
        },
        CurtainOptions {
            lock_fd: Some(0),
            ..base.clone()
        },
        CurtainOptions {
            config_path: Some("/tmp/config".into()),
            ..base.clone()
        },
    ] {
        assert!(validate_invocation_mode(&options).is_err());
    }
    assert!(
        validate_preview_mode(&CurtainOptions {
            lock: false,
            preview_png: Some("/tmp/preview".into()),
            ..base
        })
        .is_err()
    );
}

#[test]
fn removed_daemon_socket_and_ownership_flags_are_rejected() {
    for flag in [
        "--owner-gate",
        "--owner-record=/tmp/o",
        "--notify-socket=/tmp/n",
        "--daemon-socket=/tmp/a",
        "--control-socket=/tmp/c",
    ] {
        assert!(CurtainOptions::parse_args(["veila", "--lock", flag].map(String::from)).is_err());
    }
}
