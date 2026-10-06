use super::{ControlCommand, ControlOptions};

fn parse(args: &[&str]) -> anyhow::Result<ControlOptions> {
    ControlOptions::parse_args(
        std::iter::once("veila")
            .chain(args.iter().copied())
            .map(String::from),
    )
}

#[test]
fn rejects_lock_options_for_every_other_command() {
    let commands = [
        vec![],
        vec!["--version"],
        vec!["status"],
        vec!["health"],
        vec!["stop"],
        vec!["reload"],
        vec!["doctor"],
        vec!["check-config"],
        vec!["init"],
        vec!["logs"],
        vec!["theme", "list"],
        vec!["theme", "current"],
        vec!["theme", "print", "default"],
        vec!["theme", "set", "default"],
        vec!["theme", "unset"],
    ];
    for command in commands {
        for flag in ["--wait-ready", "--force-emergency-ui", "--latency-report"] {
            let args = [command.as_slice(), &[flag]].concat();
            let error = parse(&args).expect_err("lock option must fail before dispatch");
            assert_eq!(
                error.to_string(),
                format!("{flag} can only be used with `veila lock`"),
                "{args:?}"
            );
        }
    }
}

#[test]
fn version_conflicts_are_rejected_before_lock_option_validation() {
    for command in ["lock", "status", "init", "logs", "reload"] {
        let error = parse(&[command, "--version", "--wait-ready"])
            .expect_err("version and a command must fail");
        assert_eq!(error.to_string(), "use only one veila command at a time");
    }
}

#[test]
fn help_preserves_compatibility_bypass() {
    for args in [
        vec!["status", "--version", "--wait-ready", "--help"],
        vec!["--wait-ready", "-h"],
        vec!["logs", "--force-emergency-ui", "-h"],
    ] {
        assert!(parse(&args).expect("help bypasses compatibility").help);
    }
}

#[test]
fn help_does_not_bypass_syntax_errors() {
    for (args, expected) in [
        (
            vec!["lock", "status", "--help"],
            "unexpected extra argument for lock: status",
        ),
        (
            vec!["--unknown", "--help"],
            "unknown veila option: --unknown",
        ),
        (
            vec!["--latency-report=invalid", "--help"],
            "unknown latency report mode: invalid",
        ),
        (
            vec!["init", "--theme", "--help"],
            "missing value for --theme",
        ),
    ] {
        assert_eq!(
            parse(&args).expect_err("syntax error").to_string(),
            expected
        );
    }
}

#[test]
fn repeated_global_flags_and_their_order_keep_one_command() {
    for args in [
        vec!["--version", "-v", "--version"],
        vec!["--config=/tmp/one", "--version", "--config=/tmp/two", "-v"],
    ] {
        assert_eq!(
            parse(&args).expect("version").command,
            Some(ControlCommand::Version)
        );
    }
    let options = parse(&["--config=/tmp/one", "status", "--config=/tmp/two"])
        .expect("last config path wins");
    assert_eq!(
        options.config_path.as_deref(),
        Some(std::path::Path::new("/tmp/two"))
    );
    assert_eq!(options.command, Some(ControlCommand::Status));
}

#[test]
fn simple_commands_select_their_exact_variant() {
    for (args, expected) in [
        (vec!["stop"], ControlCommand::Stop),
        (vec!["status"], ControlCommand::Status),
        (vec!["health"], ControlCommand::Health),
        (vec!["theme", "list"], ControlCommand::ThemeList),
        (vec!["theme", "current"], ControlCommand::ThemeCurrent),
        (vec!["theme", "unset"], ControlCommand::ThemeUnset),
        (
            vec!["theme", "print", "default"],
            ControlCommand::ThemePrint("default".into()),
        ),
    ] {
        assert_eq!(parse(&args).expect("valid command").command, Some(expected));
    }
}

#[test]
fn no_arguments_preserve_the_empty_command() {
    assert_eq!(parse(&[]).expect("no arguments"), ControlOptions::default());
}
