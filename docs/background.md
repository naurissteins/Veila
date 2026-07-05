# Background

Veila supports several background modes under `[background]` in `config.toml`: solid colors, gradients, radial fills, layered compositions, single image files, and slideshows.

The lock screen curtain renders backgrounds in software, caches scaled buffers on disk, and preloads the next slideshow image while the current one is displayed.

## Single image

```toml
[background]
mode = "file"
path = "~/Pictures/wallpaper.jpg"
scaling = "fill" # fill | fit | center | tile | stretch
blur_strength = 0
dim_strength = 0
```

Use `[[background.outputs]]` to assign a different wallpaper per monitor:

```toml
[[background.outputs]]
name = "HDMI-A-1"
path = "~/Pictures/monitor-left.jpg"
```

## Slideshow

Enable a rotating wallpaper set from a directory and/or an explicit file list:

```toml
[background.slideshow]
enabled = true
directory = "~/Pictures/wallpapers"
# files = ["~/Pictures/extra.jpg"]
order = "sequence" # sequence | random
mode = "timed"     # timed | lock_only
change_every_seconds = 300
transition_duration_ms = 0
```

### Options

| Key | Default | Description |
|-----|---------|-------------|
| `enabled` | `true` when slideshow table is present | Turns the slideshow on or off. |
| `directory` | — | Folder scanned for `.jpg`, `.jpeg`, `.png`, and `.webp` files. |
| `files` | `[]` | Additional image paths. Entries from `directory` and `files` are merged without duplicates. |
| `order` | `sequence` | `sequence` walks files in sorted order; `random` shuffles the playlist. |
| `mode` | `timed` | `timed` rotates while the session is locked; `lock_only` picks one image per lock without timed rotation. |
| `change_every_seconds` | `300` | Interval between slideshow advances in `timed` mode (minimum 1 second). |
| `transition_duration_ms` | `0` | Crossfade length in milliseconds when switching images. Defaults to `0` (instant cut, no crossfade); set a positive value to enable the animation. Maximum `10000`. |

When `[background.slideshow]` is enabled and has sources, Veila infers `mode = "file"` even if `background.mode` is omitted.

### Crossfade behavior

In `timed` mode, Veila preloads the next wallpaper while the current one is shown. When the interval elapses:

1. The current frame is kept on screen.
2. The next image is loaded from cache or rendered in a background thread.
3. An ease-in-out crossfade runs between the previous and next frame.
4. The UI shell is not re-rendered on every animation tick; only the wallpaper blend is updated, which keeps CPU use low during transitions.

Crossfade polling runs at about 80 ms while a transition is active. Outside transitions, the lock screen keeps its normal idle refresh rate.

### Examples

Timed slideshow with a crossfade (opt-in via a positive duration):

```toml
[background.slideshow]
enabled = true
directory = "~/Pictures/wallpapers"
mode = "timed"
change_every_seconds = 120
transition_duration_ms = 1200
```

Random wallpaper on each lock, without rotation while locked:

```toml
[background.slideshow]
enabled = true
directory = "~/Pictures/wallpapers"
order = "random"
mode = "lock_only"
```

Crossfade is disabled by default; leave `transition_duration_ms` unset (or `0`)
for an instant cut:

```toml
[background.slideshow]
enabled = true
directory = "~/Pictures/wallpapers"
# transition_duration_ms = 0  # default
```

## Reloading

`veila reload` reapplies background and slideshow settings without restarting the daemon. An in-progress crossfade is cancelled on reload.

## Validation

Run `veila check-config` to validate keys under `[background]` and `[background.slideshow]`, including `transition_duration_ms`.
