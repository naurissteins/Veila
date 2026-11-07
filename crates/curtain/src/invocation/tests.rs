use super::{validate_invocation_mode, validate_preview_mode};
use crate::CurtainOptions;

#[test]
fn accepts_daemon_lock_flow_without_explicit_lock_flag() {
    let options = CurtainOptions::parse_args([
        "veila-curtain".to_string(),
        "--notify-socket=/tmp/veila.sock".to_string(),
    ])
    .expect("arguments should parse");

    assert!(validate_invocation_mode(&options).is_ok());
}

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
    assert!(
        validate_preview_mode(&parse(&[
            "--preview-png=/tmp/p.png",
            "--daemon-socket=/tmp/auth.sock",
        ]))
        .is_err()
    );
}

const OWNERSHIP_FLAGS: [&str; 5] = [
    "--owner-gate",
    "--owner-record=/tmp/owner.json",
    "--notify-socket=/tmp/notify.sock",
    "--daemon-socket=/tmp/auth.sock",
    "--control-socket=/tmp/control.sock",
];

fn ownership_options(mask: usize) -> CurtainOptions {
    CurtainOptions::parse_args(
        std::iter::once("veila")
            .chain(
                OWNERSHIP_FLAGS
                    .iter()
                    .enumerate()
                    .filter_map(|(index, flag)| (mask & (1 << index) != 0).then_some(*flag)),
            )
            .map(String::from),
    )
    .expect("valid ownership flags")
}

#[test]
fn requires_complete_gated_ownership_before_startup() {
    let allowed = [0, 4, 8, 12, 16, 20, 24, 28, 31];
    for mask in 0..32 {
        let options = CurtainOptions {
            lock: true,
            ..ownership_options(mask)
        };
        assert_eq!(
            validate_invocation_mode(&options).is_ok(),
            allowed.contains(&mask),
            "ownership flag mask {mask}",
        );
    }
}

#[test]
fn rejects_every_daemon_ownership_combination_in_preview_mode() {
    for mask in 0..32 {
        let options = CurtainOptions {
            preview_png: Some("/tmp/preview.png".into()),
            ..ownership_options(mask)
        };
        assert_eq!(
            validate_preview_mode(&options).is_ok(),
            mask == 0,
            "ownership flag mask {mask}",
        );
    }
}

#[test]
fn help_returns_before_ownership_validation_or_startup() {
    let options = CurtainOptions {
        help: true,
        ..ownership_options(1)
    };
    assert!(crate::run(options.clone()).is_ok());
    assert!(crate::run_preview(options).is_ok());
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
        validate_invocation_mode(&CurtainOptions {
            daemon_socket: Some("/tmp/auth.sock".into()),
            ..direct.clone()
        })
        .is_err()
    );
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
            daemon_socket: Some("/tmp/auth".into()),
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
