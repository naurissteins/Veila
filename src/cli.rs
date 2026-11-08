use std::path::Path;

use veila_auth::PAM_HELPER_SUBCOMMAND;
const CURTAIN_SUBCOMMAND: &str = "__curtain";
const PREWARM_SUBCOMMAND: &str = "__prewarm";

const LEGACY_DAEMON_NAME: &str = "veilad";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Lock,
    Idle,
    Control,
    Daemon,
    LegacyDaemon,
    Curtain,
    Preview,
    Prewarm,
    PamHelper,
    FontQuery,
}

// `args` keeps `argv[0]` and drops the subcommand so the mode parsers can skip one argument as before.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Invocation {
    pub(crate) mode: Mode,
    pub(crate) args: Vec<String>,
}

impl Invocation {
    pub(crate) fn parse(mut args: Vec<String>) -> Self {
        if args.first().is_some_and(|arg0| is_legacy_daemon_name(arg0)) {
            return Self {
                mode: Mode::LegacyDaemon,
                args,
            };
        }

        let mut index = 1;
        while let Some(arg) = args.get(index) {
            if arg == "--config" {
                index += 2;
                continue;
            }
            if arg == "lock" {
                args.remove(index);
                return Self {
                    mode: Mode::Lock,
                    args,
                };
            }
            if !arg.starts_with('-') {
                break;
            }
            index += 1;
        }
        let mode = match args.get(1).map(String::as_str) {
            Some("lock") => Mode::Lock,
            Some("idle") => Mode::Idle,
            Some("daemon") => Mode::Daemon,
            Some("preview") => Mode::Preview,
            Some(CURTAIN_SUBCOMMAND) => Mode::Curtain,
            Some(PREWARM_SUBCOMMAND) => Mode::Prewarm,
            Some(PAM_HELPER_SUBCOMMAND) => Mode::PamHelper,
            Some(veila_renderer::text::FONT_QUERY_SUBCOMMAND) => Mode::FontQuery,
            _ => {
                return Self {
                    mode: Mode::Control,
                    args,
                };
            }
        };
        args.remove(1);

        Self { mode, args }
    }
}

fn is_legacy_daemon_name(arg0: &str) -> bool {
    Path::new(arg0)
        .file_name()
        .is_some_and(|name| name == LEGACY_DAEMON_NAME)
}

#[cfg(test)]
mod tests {
    use super::{Invocation, Mode};

    fn parse(args: &[&str]) -> Invocation {
        Invocation::parse(args.iter().map(|arg| arg.to_string()).collect())
    }

    fn args(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn subcommands_select_modes_and_are_removed_from_arguments() {
        let cases = [
            ("lock", Mode::Lock),
            ("idle", Mode::Idle),
            ("daemon", Mode::Daemon),
            ("preview", Mode::Preview),
            ("__curtain", Mode::Curtain),
            ("__prewarm", Mode::Prewarm),
            ("__pam-helper", Mode::PamHelper),
            ("__font-query", Mode::FontQuery),
        ];

        for (subcommand, mode) in cases {
            let invocation = parse(&["/usr/bin/veila", subcommand, "--config=/tmp/c.toml"]);

            assert_eq!(invocation.mode, mode, "{subcommand}");
            assert_eq!(
                invocation.args,
                args(&["/usr/bin/veila", "--config=/tmp/c.toml"])
            );
        }
    }

    #[test]
    fn other_arguments_are_control_commands() {
        for input in [
            &["veila"][..],
            &["veila", "--config=/tmp/c.toml", "daemon"],
            &["veila", "--help"],
        ] {
            let invocation = parse(input);

            assert_eq!(invocation.mode, Mode::Control);
            assert_eq!(invocation.args, args(input));
        }
    }

    #[test]
    fn veilad_name_selects_the_deprecated_idle_alias() {
        let invocation = parse(&["/usr/bin/veilad", "--config=/tmp/c.toml"]);

        assert_eq!(invocation.mode, Mode::LegacyDaemon);
        assert_eq!(
            invocation.args,
            args(&["/usr/bin/veilad", "--config=/tmp/c.toml"])
        );
    }

    #[test]
    fn spawned_curtain_name_does_not_change_mode_detection() {
        let invocation = parse(&["veila-curtain", "__curtain", "--lock"]);

        assert_eq!(invocation.mode, Mode::Curtain);
        assert_eq!(invocation.args, args(&["veila-curtain", "--lock"]));
    }

    #[test]
    fn empty_arguments_fall_back_to_control() {
        assert_eq!(parse(&[]).mode, Mode::Control);
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    #[test]
    fn lock_preserves_global_config_and_never_routes_nested_command_values() {
        for flags in [
            vec!["--config=/tmp/config"],
            vec!["--config", "/tmp/config"],
            vec!["--wait-ready"],
        ] {
            let input = [vec!["veila"], flags.clone(), vec!["lock"]].concat();
            let parsed = Invocation::parse(input.into_iter().map(String::from).collect());
            assert_eq!(parsed.mode, Mode::Lock);
            assert_eq!(parsed.args, [vec!["veila"], flags].concat());
        }
        for input in [
            vec!["veila", "theme", "set", "lock"],
            vec!["veila", "__lock"],
            vec!["veila", "logs", "lock"],
        ] {
            assert_eq!(
                Invocation::parse(input.into_iter().map(String::from).collect()).mode,
                Mode::Control
            );
        }
    }
}
