pub(super) fn print_control_help() {
    println!(
        "\
Veila screen locker

Usage:
  veila <command> [options]

General:
  -h, --help                 Show this help text
  -v, --version              Print the local Veila version
      --config=<path>        Use a specific config file for theme/config commands
      --force-emergency-ui   Combine with `lock` to test the emergency unlock prompt
      --latency-report[=verbose]
                             Combine with `lock --wait-ready` to print startup timings

Commands:
  daemon [options]           Start the daemon (see `veila daemon --help`)
  preview --preview-png=PATH Render the lockscreen to a PNG without locking
  lock [--wait-ready]        Ask the running daemon to lock now
  status                     Print daemon runtime status
  health                     Print daemon build and platform info
  doctor                     Check local runtime prerequisites without locking
  check-config               Validate config files without starting the daemon
  init [--theme NAME]        Create config.toml with a starting theme
       [--force]             Replace an existing config.toml
  reload                     Ask the running daemon to reload config from disk
  stop                       Stop the running daemon
  logs [--follow]            Show recent systemd user journal logs
       [--file]
       [--since WHEN]
       [--lines N]
       [--daemon|--curtain|--ui|--all]

Themes:
  theme list                 List bundled themes
  theme current              Print the active theme selection
  theme print <name>         Print a theme source file
  theme set <name>           Set the active theme in config.toml
  theme unset                Remove the top-level theme key from config.toml

Notes:
  Control commands never start the daemon. Start it with `veila daemon`, a user service, or your compositor config.
  `--wait-ready` can be combined with `veila lock` to block until the secure lock is active.
  Idle and lock-before-sleep locking are configured in the [idle] section of config.toml.
"
    );
}
