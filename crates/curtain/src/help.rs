pub(crate) fn print_curtain_help() {
    println!(
        "\
Veila secure curtain (internal)

Usage:
  veila __curtain [options]

`veila daemon` starts this process for every lock. Use `veila lock` for normal locking
and `veila preview` for screenshots.

Options:
  -h, --help                         Show this help text
      --lock                         Start a real lock session without the daemon (testing only)
      --force-emergency-ui           Use the built-in emergency unlock prompt
      --latency-report[=verbose]     Send startup timing details to the daemon
      --config=<path>                Use a specific config file
      --notify-socket=<path>         Notify socket for curtain readiness
      --daemon-socket=<path>         Daemon auth IPC socket
      --control-socket=<path>        Curtain live-control IPC socket
      --initial-background-path=<path>
                                     Background image to try first
      --weather-snapshot=<payload>      Inject a weather snapshot
      --battery-snapshot=<payload>      Inject a battery snapshot
      --now-playing-snapshot=<payload>  Inject a now playing snapshot

Notes:
  Without daemon sockets and without --lock it exits to avoid accidental locks.
  Options accept both --flag=value and --flag value forms.
"
    );
}

pub(crate) fn print_preview_help() {
    println!(
        "\
Veila lockscreen preview

Usage:
  veila preview --preview-png=<path> [options]

Options:
  -h, --help                               Show this help text
      --config=<path>                      Use a specific config file
      --preview-png=<path>                 Render the scene to a PNG (required)
      --preview-size=<width>x<height>      Output size for preview rendering
      --preview-artwork=<path>             Override now playing artwork
      --preview-title=<text>               Override now playing title
      --preview-artist=<text>              Override now playing artist
      --preview-username=<text>            Override the username label
      --preview-hide-widgets               Hide widgets and the keyboard label
      --preview-hide-weather               Hide the weather widget
      --preview-hide-battery               Hide the battery widget
      --preview-hide-now-playing           Hide the now playing widget
      --preview-hide-keyboard-label        Hide the sample keyboard label
      --preview-weather-location=<text>    Override the weather location label
      --preview-weather-condition=<name>   Override the weather icon/condition
      --preview-weather-temperature=<celsius>
                                           Override the weather temperature
      --preview-battery-percent=<0-100>    Override the battery percentage
      --preview-battery-charging=<bool>    Override the battery charging state
      --preview-time=<HH:MM>               Override the clock time using the local date

Notes:
  Preview never takes a session lock.
  Options accept both --flag=value and --flag value forms.
"
    );
}
