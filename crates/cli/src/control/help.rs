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
  idle [options]             Start the optional daemonless idle watcher
  daemon [options]           Deprecated alias for `veila idle`
  preview --preview-png=PATH Render the lockscreen to a PNG without locking
  lock [--daemonize]         Lock now; stays running until unlock
  status                     Print lock and idle watcher status
  doctor                     Check local runtime prerequisites without locking
  check-config               Validate config files
  init [--theme NAME]        Create config.toml with a starting theme
       [--force]             Replace an existing config.toml
  reload                     Request lock and idle watcher config reload
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
  `veila lock` works without a resident service. Start `veila idle` for automatic locking.
  `--wait-ready` can be combined with `veila lock` to block until the secure lock is active.
  Idle and lock-before-sleep locking are configured in the [idle] section of config.toml.
"
    );
}
