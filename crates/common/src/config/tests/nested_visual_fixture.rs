use super::*;

pub(super) fn nested_visual_config() -> AppConfig {
    AppConfig::from_toml_str(
        r##"
            [visuals]
            input_border = "#111111"
            username_color = "#111111"
            foreground = "#111111"

[visuals.input]
reveal_on_interaction = true
reveal_mode = "full"
font_size = 22
font_family = "Geom"
font_weight = 600
font_style = "italic"
background_color = "#FFFFFF0D"
border_color = "#DDDDDD1F"
            width = 310
            height = 54
            radius = 10
            border_width = 0
            mask_color = "#A9C4FF"
            halign = "left"
            valign = "bottom"
            x = 28
            y = -64

            [visuals.avatar]
            size = 192
            halign = "left"
            valign = "center"
            x = 36
            y = -24
            background_color = "#FFFFFF0F"
            placeholder_padding = 28
            ring_color = "#94B2FF"
            ring_width = 0
            icon_color = "#ffffff"

            [visuals.username]
            font_family = "Geom"
            font_weight = 600
            font_style = "italic"
            color = "#FFFFFFD6"
            font_size = 28
            halign = "right"
            valign = "bottom"
            x = -40
            y = -96

            [visuals.clock]
            font_family = "Prototype"
            font_weight = 700
            font_style = "italic"
            style = "stacked"
            halign = "left"
            valign = "bottom"
            x = 20
            y = -40
            format = "12h"
            meridiem_size = 3
            meridiem_offset_x = 6
            meridiem_offset_y = -2
            color = "#FFFFFF66"
            size = 14
            gap = 20

            [visuals.date]
            font_family = "Geom"
            font_weight = 600
            font_style = "italic"
            color = "#FFFFFF66"
            size = 2
            halign = "right"
            valign = "top"
            x = -24
            y = 32

            [visuals.placeholder]
            color = "#FFFFFF99"

            [visuals.reveal]
            mode = "shown"
            text = "Press any key or click to unlock"
            color = "#D6E3FFA8"
            font_family = "Geom"
            font_weight = 500
            font_style = "italic"
            font_size = 2

            [visuals.status]
            color = "#FFE0A0E0"
pending_color = "#FFC25CBA"
rejected_color = "#DC6060EB"
halign = "right"
            valign = "top"
            x = -32
            y = 48

            [visuals.eye]
            color = "#FFFFFFB8"

            [visuals.keyboard]
            background_color = "rgba(18, 22, 30, 0.32)"
            background_size = 42
            color = "#E8EEF9AD"
            size = 3
            halign = "right"
            valign = "top"
            x = -24
            y = 29

            [visuals.battery]
            background_color = "rgba(18, 22, 30, 0.32)"
            background_size = 42
            color = "#FFFFFFB8"
            size = 18
            halign = "right"
            valign = "top"
            x = -82
            y = 29

            [visuals.caps_lock]
            enabled = true
            color = "#FFD37AA3"

            [[visuals.backdrop]]
            enabled = true
            mode = "blur"
            color = "#080A0E70"
            blur_strength = 16
            radius = 20
            border_color = "rgba(255, 255, 255, 0.18)"
            border_width = 2
            width = 520
            height = 420
            halign = "right"
            valign = "bottom"
            x = -12
            y = 16
            z = 2

            [visuals.weather.icon]
            size = 36
            opacity = 41
            halign = "right"
            valign = "bottom"
            x = -52
            y = -126

            [visuals.weather.temperature]
            font_size = 4
            font_family = "Prototype"
            font_weight = 600
            font_style = "italic"
            letter_spacing = 2
            color = "#FFFFFFB3"
            halign = "right"
            valign = "bottom"
            x = -52
            y = -80

            [visuals.weather.location]
            font_size = 2
            font_family = "Geom"
            font_weight = 500
            font_style = "italic"
            color = "#D6E3FF62"
            halign = "right"
            valign = "bottom"
            x = -52
            y = -52

            [visuals.now_playing]
            fade_duration_ms = 320

            [visuals.now_playing.artwork]
            enabled = true
            size = 64
            radius = 16
            opacity = 61
            halign = "right"
            valign = "bottom"
            x = -274
            y = -46

            [visuals.now_playing.artist]
            width = 198
            color = "#C8D4EC63"
            font_family = "Prototype"
            font_size = 1
            font_weight = 500
            font_style = "italic"
            halign = "right"
            valign = "bottom"
            x = -58
            y = -78

            [visuals.now_playing.title]
            width = 198
            color = "#F8FBFFD0"
            font_family = "Geom"
            font_size = 2
            font_weight = 700
            font_style = "italic"
            halign = "right"
            valign = "bottom"
            x = -58
            y = -46

            [visuals.outputs]
            ui_mode = "single"
            ui_output = "DP-1"

            [visuals.palette]
            foreground = "rgba(255, 255, 255, 0.1)"
            muted = "rgba(255, 255, 255, 0.9)"
            pending = "rgba(255, 255, 255, 0.9)"
            rejected = "rgba(255, 255, 255, 0.9)"
        "##,
    )
    .expect("nested visual config should parse")
}
