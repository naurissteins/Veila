use veila_common::LayerMode;
use veila_renderer::{SoftwareBuffer, layer::BackdropLayerMode, text::TextBlock};

use super::super::{NowPlayingWidgetData, ShellState};
use super::{
    NOW_PLAYING_BOTTOM_PADDING, NOW_PLAYING_MAX_TEXT_WIDTH, NOW_PLAYING_MIN_TEXT_WIDTH,
    NOW_PLAYING_RIGHT_PADDING, SceneLayout, widgets,
    widgets::{NowPlayingBackgroundStyle, NowPlayingWidget},
};

impl ShellState {
    pub(super) fn render_now_playing_widget(
        &self,
        buffer: &mut SoftwareBuffer,
        layout: &SceneLayout,
    ) {
        let fade_progress = self.now_playing_fade_progress();
        if !self.theme.now_playing_enabled
            || (self.now_playing.is_none()
                && self
                    .now_playing_transition
                    .as_ref()
                    .and_then(|transition| transition.previous.as_ref())
                    .is_none())
        {
            return;
        }

        let artwork_size = self
            .theme
            .now_playing_artwork_size
            .unwrap_or(56)
            .clamp(32, 160);
        let content_gap = self
            .theme
            .now_playing_content_gap
            .unwrap_or(widgets::NOW_PLAYING_CONTENT_GAP)
            .clamp(0, 96);
        let now_playing_width = self
            .theme
            .now_playing_width
            .map(|width| width.clamp(96, 640));
        let text_max_width = now_playing_width
            .map(|width| {
                (width - artwork_size - content_gap).max(NOW_PLAYING_MIN_TEXT_WIDTH) as u32
            })
            .unwrap_or(NOW_PLAYING_MAX_TEXT_WIDTH);
        let base_bottom_padding = self
            .theme
            .now_playing_bottom_padding
            .unwrap_or(NOW_PLAYING_BOTTOM_PADDING)
            .clamp(0, 512);
        let bottom_padding = if self.theme.weather_alignment
            == veila_common::WeatherAlignment::Right
            && layout
                .model
                .sections_for_role(super::model::LayoutRole::Footer)
                .next()
                .is_some()
        {
            (buffer.size().height as i32 - layout.anchors.footer_y + 24).max(base_bottom_padding)
        } else {
            base_bottom_padding
        };

        if let Some(transition) = self.now_playing_transition.as_ref()
            && let Some(previous) = transition.previous.as_ref()
        {
            let fade_out = 100u8.saturating_sub(fade_progress.unwrap_or(100));
            self.draw_now_playing_snapshot(
                buffer,
                previous,
                artwork_size,
                content_gap,
                now_playing_width,
                text_max_width,
                bottom_padding,
                fade_out,
            );
        }

        if let Some(now_playing) = self.now_playing.as_ref() {
            let fade_in = fade_progress.unwrap_or(100);
            self.draw_now_playing_snapshot(
                buffer,
                now_playing,
                artwork_size,
                content_gap,
                now_playing_width,
                text_max_width,
                bottom_padding,
                fade_in,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_now_playing_snapshot(
        &self,
        buffer: &mut SoftwareBuffer,
        now_playing: &NowPlayingWidgetData,
        artwork_size: i32,
        content_gap: i32,
        now_playing_width: Option<i32>,
        text_max_width: u32,
        bottom_padding: i32,
        opacity_scale: u8,
    ) {
        let mut text_layout_cache = self.text_layout_cache.borrow_mut();
        let title = apply_block_opacity(
            text_layout_cache.now_playing_title_block(
                &now_playing.title,
                self.now_playing_title_text_style(),
                text_max_width,
            ),
            opacity_scale,
        );
        let artist = now_playing.artist.as_deref().map(|artist| {
            apply_block_opacity(
                text_layout_cache.now_playing_artist_block(
                    artist,
                    self.now_playing_artist_text_style(),
                    text_max_width,
                ),
                opacity_scale,
            )
        });

        widgets::draw_now_playing_widget(
            buffer,
            NowPlayingWidget {
                artwork: now_playing.artwork.as_ref(),
                title: &title,
                artist: artist.as_ref(),
                background: self.now_playing_background_style(opacity_scale),
                artwork_opacity: combine_optional_opacity(
                    self.theme.now_playing_artwork_opacity,
                    opacity_scale,
                ),
                artwork_size,
                artwork_radius: self
                    .theme
                    .now_playing_artwork_radius
                    .unwrap_or(12)
                    .clamp(0, 80),
                width: now_playing_width,
                content_gap,
                text_gap: self
                    .theme
                    .now_playing_text_gap
                    .unwrap_or(widgets::NOW_PLAYING_TEXT_GAP)
                    .clamp(0, 64),
                right_padding: self
                    .theme
                    .now_playing_right_padding
                    .unwrap_or(NOW_PLAYING_RIGHT_PADDING)
                    .clamp(0, 512),
                bottom_padding,
                right_offset: self
                    .theme
                    .now_playing_right_offset
                    .unwrap_or(0)
                    .clamp(-512, 512),
                bottom_offset: self
                    .theme
                    .now_playing_bottom_offset
                    .unwrap_or(0)
                    .clamp(-512, 512),
            },
        );
    }

    fn now_playing_background_style(&self, opacity_scale: u8) -> Option<NowPlayingBackgroundStyle> {
        if !self.theme.now_playing_background_enabled {
            return None;
        }

        let mode = match self.theme.now_playing_background_mode {
            LayerMode::Solid => BackdropLayerMode::Solid,
            LayerMode::Blur => BackdropLayerMode::Blur,
        };
        let mut color = self.theme.now_playing_background_color;
        color.alpha = ((u16::from(color.alpha) * u16::from(opacity_scale.min(100))) / 100) as u8;
        let border_color = self
            .theme
            .now_playing_background_border_color
            .map(|mut color| {
                color.alpha =
                    ((u16::from(color.alpha) * u16::from(opacity_scale.min(100))) / 100) as u8;
                color
            });

        Some(NowPlayingBackgroundStyle {
            mode,
            color,
            blur_radius: self
                .theme
                .now_playing_background_blur_radius
                .unwrap_or(12)
                .min(24),
            radius: self
                .theme
                .now_playing_background_radius
                .unwrap_or(18)
                .clamp(0, 80),
            padding_x: self
                .theme
                .now_playing_background_padding_x
                .unwrap_or(18)
                .clamp(0, 160),
            padding_y: self
                .theme
                .now_playing_background_padding_y
                .unwrap_or(12)
                .clamp(0, 160),
            border_color,
            border_width: self
                .theme
                .now_playing_background_border_width
                .unwrap_or(0)
                .clamp(0, 12),
        })
    }
}

fn apply_block_opacity(mut block: TextBlock, opacity_scale: u8) -> TextBlock {
    block.style.color = block.style.color.with_alpha(
        ((u16::from(block.style.color.alpha) * u16::from(opacity_scale.min(100))) / 100) as u8,
    );
    block
}

fn combine_optional_opacity(base: Option<u8>, scale: u8) -> Option<u8> {
    Some(((u16::from(base.unwrap_or(100).min(100)) * u16::from(scale.min(100))) / 100) as u8)
}
