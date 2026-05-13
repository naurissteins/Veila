use veila_renderer::{FrameSize, PixelBuffer, shape::Rect, text::TextBlock};

use super::super::{NowPlayingWidgetData, ShellState};
use super::{NOW_PLAYING_MAX_TEXT_WIDTH, NOW_PLAYING_MIN_TEXT_WIDTH, SceneLayout, TextLayoutCache};

const NOW_PLAYING_TEXT_WIDTH_CAP: i32 = 640;

impl ShellState {
    pub(super) fn render_now_playing_widget(
        &self,
        buffer: &mut impl PixelBuffer,
        _layout: &SceneLayout,
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

        if let Some(transition) = self.now_playing_transition.as_ref()
            && let Some(previous) = transition.previous.as_ref()
        {
            let fade_out = 100u8.saturating_sub(fade_progress.unwrap_or(100));
            self.draw_now_playing_snapshot(buffer, previous, fade_out);
        }

        if let Some(now_playing) = self.now_playing.as_ref() {
            let fade_in = fade_progress.unwrap_or(100);
            self.draw_now_playing_snapshot(buffer, now_playing, fade_in);
        }
    }

    fn draw_now_playing_snapshot(
        &self,
        buffer: &mut impl PixelBuffer,
        now_playing: &NowPlayingWidgetData,
        fade_percent: u8,
    ) {
        let Some(layout) =
            self.now_playing_snapshot_layout(buffer.size(), now_playing, fade_percent)
        else {
            return;
        };

        if let Some(artwork) = layout.artwork.as_ref() {
            artwork.asset.draw(
                buffer,
                artwork.rect.x,
                artwork.rect.y,
                artwork.rect.width as u32,
                artwork.rect.height as u32,
                artwork.radius,
                artwork.opacity,
            );
        }

        if let Some(artist) = layout.artist.as_ref() {
            artist.block.draw(buffer, artist.rect.x, artist.rect.y);
        }

        if let Some(title) = layout.title.as_ref() {
            title.block.draw(buffer, title.rect.x, title.rect.y);
        }
    }

    fn now_playing_snapshot_layout<'a>(
        &self,
        size: FrameSize,
        now_playing: &'a NowPlayingWidgetData,
        fade_percent: u8,
    ) -> Option<NowPlayingSnapshotLayout<'a>> {
        let mut text_layout_cache = self.text_layout_cache.borrow_mut();
        let title = if self.theme.now_playing_title_enabled {
            self.now_playing_title_part(
                size,
                &mut text_layout_cache,
                &now_playing.title,
                fade_percent,
            )
        } else {
            None
        };
        let artist = if self.theme.now_playing_artist_enabled {
            now_playing.artist.as_deref().and_then(|artist| {
                self.now_playing_artist_part(size, &mut text_layout_cache, artist, fade_percent)
            })
        } else {
            None
        };
        let artwork = (self.theme.now_playing_artwork_enabled)
            .then(|| self.now_playing_artwork_part(size, now_playing, fade_percent))
            .flatten();

        if artwork.is_none() && artist.is_none() && title.is_none() {
            return None;
        }

        Some(NowPlayingSnapshotLayout {
            artwork,
            artist,
            title,
        })
    }

    fn now_playing_artwork_part<'a>(
        &self,
        size: FrameSize,
        now_playing: &'a NowPlayingWidgetData,
        fade_percent: u8,
    ) -> Option<NowPlayingArtworkPart<'a>> {
        let asset = now_playing.artwork.as_ref()?;
        let position = self.theme.now_playing_artwork_position?;
        let artwork_size = self
            .theme
            .now_playing_artwork_size
            .unwrap_or(44)
            .clamp(32, 160);
        let rect = self.positioned_rect(size, position, artwork_size, artwork_size);

        Some(NowPlayingArtworkPart {
            asset,
            rect,
            radius: self
                .theme
                .now_playing_artwork_radius
                .unwrap_or(8)
                .clamp(0, 80),
            opacity: combine_optional_fade(self.theme.now_playing_artwork_opacity, fade_percent),
        })
    }

    fn now_playing_artist_part(
        &self,
        size: FrameSize,
        text_layout_cache: &mut TextLayoutCache,
        artist: &str,
        fade_percent: u8,
    ) -> Option<NowPlayingTextPart> {
        let position = self.theme.now_playing_artist_position?;
        let box_width = self.now_playing_text_width(self.theme.now_playing_artist_width);
        let block = apply_text_fade(
            text_layout_cache.now_playing_artist_block(
                artist,
                self.now_playing_artist_text_style(),
                box_width as u32,
            ),
            fade_percent,
        );
        let rect = self.positioned_rect(size, position, box_width, block.height as i32);

        Some(NowPlayingTextPart {
            rect: Rect::new(rect.x, rect.y, block.width as i32, block.height as i32),
            block,
        })
    }

    fn now_playing_title_part(
        &self,
        size: FrameSize,
        text_layout_cache: &mut TextLayoutCache,
        title: &str,
        fade_percent: u8,
    ) -> Option<NowPlayingTextPart> {
        let position = self.theme.now_playing_title_position?;
        let box_width = self.now_playing_text_width(self.theme.now_playing_title_width);
        let block = apply_text_fade(
            text_layout_cache.now_playing_title_block(
                title,
                self.now_playing_title_text_style(),
                box_width as u32,
            ),
            fade_percent,
        );
        let rect = self.positioned_rect(size, position, box_width, block.height as i32);

        Some(NowPlayingTextPart {
            rect: Rect::new(rect.x, rect.y, block.width as i32, block.height as i32),
            block,
        })
    }

    fn now_playing_text_width(&self, configured_width: Option<i32>) -> i32 {
        let scale = self.render_scale.max(1) as i32;
        let default_width = (NOW_PLAYING_MAX_TEXT_WIDTH as i32).saturating_mul(scale);
        let min_width = NOW_PLAYING_MIN_TEXT_WIDTH.saturating_mul(scale);
        let max_width = NOW_PLAYING_TEXT_WIDTH_CAP.saturating_mul(scale);

        configured_width
            .unwrap_or(default_width)
            .clamp(min_width, max_width)
    }
}

#[derive(Debug)]
struct NowPlayingSnapshotLayout<'a> {
    artwork: Option<NowPlayingArtworkPart<'a>>,
    artist: Option<NowPlayingTextPart>,
    title: Option<NowPlayingTextPart>,
}

#[derive(Debug)]
struct NowPlayingArtworkPart<'a> {
    asset: &'a veila_renderer::cover::CoverArtAsset,
    rect: Rect,
    radius: i32,
    opacity: Option<u8>,
}

#[derive(Debug)]
struct NowPlayingTextPart {
    rect: Rect,
    block: TextBlock,
}

fn apply_text_fade(mut block: TextBlock, fade_percent: u8) -> TextBlock {
    block.style.color = block.style.color.with_alpha(
        ((u16::from(block.style.color.alpha) * u16::from(fade_percent.min(100))) / 100) as u8,
    );
    block
}

fn combine_optional_fade(base: Option<u8>, fade_percent: u8) -> Option<u8> {
    Some(((u16::from(base.unwrap_or(100).min(100)) * u16::from(fade_percent.min(100))) / 100) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::ShellTheme;
    use veila_common::{NowPlayingSnapshot, WeatherUnit};

    #[test]
    fn artist_can_render_without_title() {
        let shell = ShellState::new_with_username_and_widgets(
            ShellTheme {
                now_playing_enabled: true,
                now_playing_artist_enabled: true,
                now_playing_title_enabled: false,
                ..ShellTheme::default()
            },
            None,
            None,
            None,
            true,
            None,
            None,
            WeatherUnit::default(),
            None,
            Some(NowPlayingSnapshot {
                title: String::from("Track"),
                artist: Some(String::from("Artist")),
                artwork_path: None,
                fetched_at_unix: 0,
            }),
        );

        let layout = shell
            .now_playing_snapshot_layout(
                FrameSize::new(1280, 720),
                shell.now_playing.as_ref().expect("now playing snapshot"),
                100,
            )
            .expect("artist-only layout should render");

        assert!(layout.title.is_none());
        assert!(layout.artist.is_some());
    }

    #[test]
    fn fallback_title_width_scales_with_output_scale() {
        let title = "VHS DREAMS '88 | 2 Hour Synthwave, Chillwave & Retrowave Mix";
        let theme = ShellTheme {
            now_playing_enabled: true,
            now_playing_artist_enabled: false,
            now_playing_title_enabled: true,
            now_playing_title_width: None,
            now_playing_title_font_size: Some(16),
            ..ShellTheme::default()
        };
        let shell = ShellState::new_with_username_and_widgets(
            theme.clone(),
            None,
            None,
            None,
            true,
            None,
            None,
            WeatherUnit::default(),
            None,
            Some(NowPlayingSnapshot {
                title: title.to_owned(),
                artist: None,
                artwork_path: None,
                fetched_at_unix: 0,
            }),
        );
        let mut scaled_shell = ShellState::new_with_username_and_widgets(
            theme.scaled_for_render(2),
            None,
            None,
            None,
            true,
            None,
            None,
            WeatherUnit::default(),
            None,
            Some(NowPlayingSnapshot {
                title: title.to_owned(),
                artist: None,
                artwork_path: None,
                fetched_at_unix: 0,
            }),
        );
        scaled_shell.render_scale = 2;

        let title_line = shell
            .now_playing_snapshot_layout(
                FrameSize::new(2560, 1440),
                shell.now_playing.as_ref().expect("now playing snapshot"),
                100,
            )
            .and_then(|layout| layout.title)
            .and_then(|title| title.block.lines.into_iter().next())
            .expect("title should render");
        let scaled_title_line = scaled_shell
            .now_playing_snapshot_layout(
                FrameSize::new(5120, 2880),
                scaled_shell
                    .now_playing
                    .as_ref()
                    .expect("now playing snapshot"),
                100,
            )
            .and_then(|layout| layout.title)
            .and_then(|title| title.block.lines.into_iter().next())
            .expect("scaled title should render");

        assert_eq!(title_line, scaled_title_line);
    }
}
