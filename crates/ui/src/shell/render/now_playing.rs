use veila_common::LayerMode;
use veila_renderer::{
    FrameSize, SoftwareBuffer, layer::BackdropLayerMode, shape::Rect, text::TextBlock,
};

use super::super::{NowPlayingWidgetData, ShellState};
use super::{
    NOW_PLAYING_MAX_TEXT_WIDTH, NOW_PLAYING_MIN_TEXT_WIDTH, SceneLayout, TextLayoutCache,
    layout::{anchored_block_x, anchored_block_y},
    widgets,
    widgets::NowPlayingBackgroundStyle,
};

impl ShellState {
    pub(super) fn render_now_playing_widget(
        &self,
        buffer: &mut SoftwareBuffer,
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
        buffer: &mut SoftwareBuffer,
        now_playing: &NowPlayingWidgetData,
        opacity_scale: u8,
    ) {
        let Some(layout) =
            self.now_playing_snapshot_layout(buffer.size(), now_playing, opacity_scale)
        else {
            return;
        };

        if let (Some(background), Some(content_rect)) = (
            self.now_playing_background_style(opacity_scale),
            layout.content_rect(),
        ) {
            widgets::draw_now_playing_background(buffer, content_rect, background);
        }

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
        opacity_scale: u8,
    ) -> Option<NowPlayingSnapshotLayout<'a>> {
        let mut text_layout_cache = self.text_layout_cache.borrow_mut();
        let title = self.now_playing_title_part(
            size,
            &mut text_layout_cache,
            &now_playing.title,
            opacity_scale,
        )?;
        let artist = now_playing.artist.as_deref().and_then(|artist| {
            self.now_playing_artist_part(size, &mut text_layout_cache, artist, opacity_scale)
        });
        let artwork = (self.theme.now_playing_artwork_enabled)
            .then(|| self.now_playing_artwork_part(size, now_playing, opacity_scale))
            .flatten();

        Some(NowPlayingSnapshotLayout {
            artwork,
            artist,
            title: Some(title),
        })
    }

    fn now_playing_artwork_part<'a>(
        &self,
        size: FrameSize,
        now_playing: &'a NowPlayingWidgetData,
        opacity_scale: u8,
    ) -> Option<NowPlayingArtworkPart<'a>> {
        let asset = now_playing.artwork.as_ref()?;
        let position = self.theme.now_playing_artwork_position?;
        let artwork_size = self
            .theme
            .now_playing_artwork_size
            .unwrap_or(44)
            .clamp(32, 160);
        let x = anchored_block_x(size.width as i32, artwork_size, position.halign, position.x);
        let y = anchored_block_y(
            size.height as i32,
            artwork_size,
            position.valign,
            position.y,
        );

        Some(NowPlayingArtworkPart {
            asset,
            rect: Rect::new(x, y, artwork_size, artwork_size),
            radius: self
                .theme
                .now_playing_artwork_radius
                .unwrap_or(8)
                .clamp(0, 80),
            opacity: combine_optional_opacity(
                self.theme.now_playing_artwork_opacity,
                opacity_scale,
            ),
        })
    }

    fn now_playing_artist_part(
        &self,
        size: FrameSize,
        text_layout_cache: &mut TextLayoutCache,
        artist: &str,
        opacity_scale: u8,
    ) -> Option<NowPlayingTextPart> {
        let position = self.theme.now_playing_artist_position?;
        let box_width = self
            .theme
            .now_playing_artist_width
            .unwrap_or(NOW_PLAYING_MAX_TEXT_WIDTH as i32)
            .clamp(NOW_PLAYING_MIN_TEXT_WIDTH, 640);
        let block = apply_block_opacity(
            text_layout_cache.now_playing_artist_block(
                artist,
                self.now_playing_artist_text_style(),
                box_width as u32,
            ),
            opacity_scale,
        );
        let x = anchored_block_x(size.width as i32, box_width, position.halign, position.x);
        let y = anchored_block_y(
            size.height as i32,
            block.height as i32,
            position.valign,
            position.y,
        );

        Some(NowPlayingTextPart {
            rect: Rect::new(x, y, block.width as i32, block.height as i32),
            block,
        })
    }

    fn now_playing_title_part(
        &self,
        size: FrameSize,
        text_layout_cache: &mut TextLayoutCache,
        title: &str,
        opacity_scale: u8,
    ) -> Option<NowPlayingTextPart> {
        let position = self.theme.now_playing_title_position?;
        let box_width = self
            .theme
            .now_playing_title_width
            .unwrap_or(NOW_PLAYING_MAX_TEXT_WIDTH as i32)
            .clamp(NOW_PLAYING_MIN_TEXT_WIDTH, 640);
        let block = apply_block_opacity(
            text_layout_cache.now_playing_title_block(
                title,
                self.now_playing_title_text_style(),
                box_width as u32,
            ),
            opacity_scale,
        );
        let x = anchored_block_x(size.width as i32, box_width, position.halign, position.x);
        let y = anchored_block_y(
            size.height as i32,
            block.height as i32,
            position.valign,
            position.y,
        );

        Some(NowPlayingTextPart {
            rect: Rect::new(x, y, block.width as i32, block.height as i32),
            block,
        })
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

#[derive(Debug)]
struct NowPlayingSnapshotLayout<'a> {
    artwork: Option<NowPlayingArtworkPart<'a>>,
    artist: Option<NowPlayingTextPart>,
    title: Option<NowPlayingTextPart>,
}

impl NowPlayingSnapshotLayout<'_> {
    fn content_rect(&self) -> Option<Rect> {
        let mut rects = self
            .artwork
            .as_ref()
            .map(|artwork| artwork.rect)
            .into_iter()
            .chain(self.artist.as_ref().map(|artist| artist.rect))
            .chain(self.title.as_ref().map(|title| title.rect));

        let first = rects.next()?;
        Some(rects.fold(first, union_rect))
    }
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

fn union_rect(left: Rect, right: Rect) -> Rect {
    let x = left.x.min(right.x);
    let y = left.y.min(right.y);
    let right_edge = (left.x + left.width).max(right.x + right.width);
    let bottom_edge = (left.y + left.height).max(right.y + right.height);
    Rect::new(x, y, right_edge - x, bottom_edge - y)
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
