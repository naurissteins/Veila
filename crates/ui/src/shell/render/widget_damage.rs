use veila_common::BackdropShowWhen;
use veila_renderer::{FrameSize, PixelBuffer, shape::Rect};

use super::{RenderContext, layout::hero_block_x, model::SceneWidget};

mod auth;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetKind {
    Header,
    Media,
    Indicators,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetDamage {
    Full,
    Skip,
    Region(Rect),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetRegions {
    size: FrameSize,
    header: Option<Rect>,
    media: Option<Rect>,
    indicators: Option<Rect>,
    auth: Option<Rect>,
    weather: Option<Rect>,
    partial_unsafe: bool,
}

impl WidgetRegions {
    pub fn damage_since(self, previous: Self, widget: WidgetKind) -> WidgetDamage {
        if self.size != previous.size || self.partial_unsafe || previous.partial_unsafe {
            return WidgetDamage::Full;
        }

        let (old, new, others) = match widget {
            WidgetKind::Header => (
                previous.header,
                self.header,
                [
                    (previous.media, self.media),
                    (previous.indicators, self.indicators),
                    (previous.auth, self.auth),
                    (previous.weather, self.weather),
                ],
            ),
            WidgetKind::Media => (
                previous.media,
                self.media,
                [
                    (previous.header, self.header),
                    (previous.indicators, self.indicators),
                    (previous.auth, self.auth),
                    (previous.weather, self.weather),
                ],
            ),
            WidgetKind::Indicators => (
                previous.indicators,
                self.indicators,
                [
                    (previous.header, self.header),
                    (previous.media, self.media),
                    (previous.auth, self.auth),
                    (previous.weather, self.weather),
                ],
            ),
        };
        if others.iter().any(|(old, new)| old != new) {
            return WidgetDamage::Full;
        }

        let Some(region) = merge(old, new) else {
            return WidgetDamage::Skip;
        };
        let damage = region
            .inflated(12)
            .clipped_to(self.size.width as i32, self.size.height as i32);
        if damage.is_empty() {
            return WidgetDamage::Skip;
        }
        if others
            .iter()
            .filter_map(|(_, current)| *current)
            .any(|other| overlaps(damage, other))
        {
            WidgetDamage::Full
        } else {
            WidgetDamage::Region(damage)
        }
    }
}

impl RenderContext<'_> {
    pub(super) fn widget_regions(&self, size: FrameSize) -> WidgetRegions {
        let layout = self.scene_layout(size);
        let partial_unsafe = self.shell.emergency_active()
            || self.shell.preview_grid_enabled
            || self.theme.backdrops.iter().any(|backdrop| {
                backdrop.show_when != BackdropShowWhen::Always
                    && self.shell.backdrop_visible(backdrop)
                    && backdrop.rotate != 0
            });
        WidgetRegions {
            size,
            header: self.header_region(size, &layout),
            media: merge(
                self.now_playing_region(size),
                self.dynamic_backdrop_region(size, BackdropShowWhen::NowPlaying),
            ),
            indicators: merge(
                self.indicator_region(size),
                self.dynamic_backdrop_region(size, BackdropShowWhen::Battery),
            ),
            auth: self.auth_dirty_rect(size),
            weather: merge(
                self.weather_region(size, &layout),
                self.dynamic_backdrop_region(size, BackdropShowWhen::Weather),
            ),
            partial_unsafe,
        }
    }

    pub(super) fn render_widget(&self, buffer: &mut impl PixelBuffer, widget: WidgetKind) {
        match widget {
            WidgetKind::Header => self.render_header_widget_group(buffer),
            WidgetKind::Media => {
                self.render_widget_backdrops(buffer, BackdropShowWhen::NowPlaying);
                let layout = self.scene_layout(buffer.size());
                self.render_now_playing_widget(buffer, &layout);
            }
            WidgetKind::Indicators => {
                self.render_widget_backdrops(buffer, BackdropShowWhen::Battery);
                self.render_top_right_indicators(buffer);
            }
        }
    }

    fn header_region(&self, size: FrameSize, layout: &super::SceneLayout) -> Option<Rect> {
        let mut region = None;
        let mut y = layout.anchors.hero_y;
        for section in layout
            .model
            .sections_for_role(super::model::LayoutRole::Hero)
        {
            let backdrop_center = self
                .theme
                .clock_center_in_layer
                .then(|| self.first_backdrop_center_x(size))
                .flatten();
            let next = match &section.widget {
                SceneWidget::Clock(clock) => Some(Rect::new(
                    hero_block_x(
                        size.width as i32,
                        clock.width(),
                        self.theme.clock_alignment,
                        backdrop_center,
                        self.theme.clock_offset_x,
                    ),
                    y,
                    clock.width(),
                    clock.height(),
                )),
                SceneWidget::Date(date) => Some(Rect::new(
                    hero_block_x(
                        size.width as i32,
                        date.width as i32,
                        self.theme.clock_alignment,
                        backdrop_center,
                        self.theme.clock_offset_x,
                    ),
                    y,
                    date.width as i32,
                    date.height as i32,
                )),
                _ => None,
            };
            region = merge(region, next);
            y += section.height(layout.metrics, &self.shell.status) + section.gap_after;
        }
        if let Some((rect, _)) = layout.floating_clock.as_ref() {
            region = merge(region, Some(*rect));
        }
        if let Some((rect, _)) = layout.floating_date.as_ref() {
            region = merge(region, Some(*rect));
        }
        region
    }

    fn dynamic_backdrop_region(&self, size: FrameSize, when: BackdropShowWhen) -> Option<Rect> {
        self.theme
            .backdrops
            .iter()
            .filter(|backdrop| backdrop.show_when == when && self.shell.backdrop_visible(backdrop))
            .map(|backdrop| self.backdrop_rect(size, backdrop.clone()))
            .fold(None, |region, rect| merge(region, Some(rect)))
    }

    fn weather_region(&self, size: FrameSize, layout: &super::SceneLayout) -> Option<Rect> {
        let weather = layout.floating_weather.as_ref()?;
        let mut region = None;
        if let (Some(icon), Some(position)) = (weather.icon, self.theme.weather_icon_position) {
            region = merge(
                region,
                Some(self.positioned_rect(size, position, icon.size, icon.size)),
            );
        }
        if let (Some(block), Some(position)) = (
            weather.temperature.as_ref(),
            self.theme.weather_temperature_position,
        ) {
            region = merge(
                region,
                Some(self.positioned_rect(size, position, block.width as i32, block.height as i32)),
            );
        }
        if let (Some(block), Some(position)) = (
            weather.location.as_ref(),
            self.theme.weather_location_position,
        ) {
            region = merge(
                region,
                Some(self.positioned_rect(size, position, block.width as i32, block.height as i32)),
            );
        }
        region
    }
}

pub(super) fn merge(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.union(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

#[cfg(test)]
mod tests;
