use super::*;

pub(super) fn nested_visual_config() -> AppConfig {
    AppConfig::from_toml_str(
        r##"
            [visuals]
            input_border = "#111111"
            username_color = "#111111"
            clock_gap = 6
            foreground = "#111111"

[visuals.input]
alignment = "bottom-left"
center_in_layer = true
horizontal_padding = 64
vertical_padding = 56
offset_x = 14
offset_y = -18
font_size = 3
font_family = "Geom"
font_weight = 600
font_style = "italic"
background_color = "#FFFFFF"
background_opacity = 5
border_color = "#DDDDDD"
            border_opacity = 12
            width = 310
            height = 54
            radius = 10
            border_width = 0
            mask_color = "#A9C4FF"

            [visuals.avatar]
            size = 192
            offset_y = 12
            gap = 14
            background_color = "#ffffff"
            background_opacity = 6
            placeholder_padding = 28
            ring_color = "#94B2FF"
            ring_width = 0
            icon_color = "#ffffff"

            [visuals.username]
            font_family = "Geom"
            font_weight = 600
            font_style = "italic"
            color = "#ffffff"
            opacity = 84
            size = 4
            offset_y = -10
            gap = 28

            [visuals.clock]
            font_family = "Prototype"
            font_weight = 700
            font_style = "italic"
            style = "stacked"
            alignment = "top-right"
            center_in_layer = true
            offset_x = -18
            offset_y = 14
            format = "12h"
            meridiem_size = 3
            meridiem_offset_x = 6
            meridiem_offset_y = -2
            color = "#ffffff"
            opacity = 40
            size = 14
            gap = 20

            [visuals.date]
            font_family = "Geom"
            font_weight = 600
            font_style = "italic"
            color = "#ffffff"
            opacity = 40
            size = 2

            [visuals.placeholder]
            color = "#ffffff"
            opacity = 60

            [visuals.status]
            color = "#FFE0A0"
            opacity = 88
            gap = 18

            [visuals.eye]
            color = "#ffffff"
            opacity = 72

            [visuals.keyboard]
            background_color = "rgba(18, 22, 30, 0.32)"
            background_size = 42
            color = "#E8EEF9"
            opacity = 68
            size = 3
            top_offset = -12
            right_offset = 8

            [visuals.battery]
            background_color = "rgba(18, 22, 30, 0.32)"
            background_size = 42
            color = "#FFFFFF"
            opacity = 72
            size = 18
            top_offset = -12
            right_offset = 0
            gap = 8

            [visuals.layer]
            enabled = true
            mode = "blur"
            style = "diagonal"
            alignment = "right"
            width = 520
            height = 420
            vertical_alignment = "bottom"
            offset_x = -12
            offset_y = 16
            left_margin = 24
            right_margin = 36
            top_margin = 18
            bottom_margin = 22
            color = "#080A0E"
            opacity = 44
            blur_radius = 16
            radius = 20
            border_color = "rgba(255, 255, 255, 0.18)"
            border_width = 2

            [visuals.weather]
            size = 3
            opacity = 62
            icon_opacity = 41
            temperature_opacity = 77
            location_opacity = 53
            temperature_color = "#FFFFFF"
            location_color = "#D6E3FF"
            temperature_font_family = "Prototype"
            temperature_font_weight = 600
            temperature_font_style = "italic"
            temperature_letter_spacing = 2
            location_font_family = "Geom"
            location_font_weight = 500
            location_font_style = "italic"
            temperature_size = 4
            location_size = 2
            icon_size = 36
            icon_gap = 10
            location_gap = 3
            alignment = "right"
            left_offset = 12
            bottom_offset = -6
            left_padding = 56
            horizontal_padding = 64
            bottom_padding = 72

            [visuals.now_playing]
            fade_duration_ms = 320
            opacity = 72
            title_opacity = 88
            artist_opacity = 54
            artwork_opacity = 61
            title_color = "#F8FBFF"
            artist_color = "#C8D4EC"
            title_font_family = "Geom"
            artist_font_family = "Prototype"
            title_font_weight = 700
            artist_font_weight = 500
            title_font_style = "italic"
            artist_font_style = "italic"
            title_size = 2
            artist_size = 1
            width = 280
            content_gap = 18
            text_gap = 10
            artwork_size = 64
            artwork_radius = 16
            right_padding = 52
            bottom_padding = 56
            right_offset = -6
            bottom_offset = 10

            [visuals.now_playing.background]
            enabled = true
            mode = "blur"
            color = "#000000"
            opacity = 24
            blur_radius = 12
            radius = 18
            padding_x = 20
            padding_y = 14
            border_color = "#FFFFFF"
            border_opacity = 10
            border_width = 1

            [visuals.layout]
            header_top_offset = -12
            auth_stack_offset = 0
            identity_gap = 26
            center_stack_order = "auth-hero"
            center_stack_style = "identity-hero-input"

            [visuals.palette]
            foreground = "rgba(255, 255, 255, 0.1)"
            muted = "rgba(255, 255, 255, 0.9)"
            pending = "rgba(255, 255, 255, 0.9)"
            rejected = "rgba(255, 255, 255, 0.9)"
        "##,
    )
    .expect("nested visual config should parse")
}
