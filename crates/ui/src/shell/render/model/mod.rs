mod standard;
#[cfg(test)]
mod tests;

use veila_common::WeatherAlignment;
use veila_renderer::icon::WeatherIcon;
use veila_renderer::text::TextBlock;

use super::{super::ShellStatus, layout::SceneMetrics};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTextBlocks {
    pub clock: TextBlock,
    pub date: TextBlock,
    pub username: Option<TextBlock>,
    pub placeholder: TextBlock,
    pub status: Option<TextBlock>,
    pub weather: Option<SceneWeatherBlocks>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneWeatherBlocks {
    pub temperature: TextBlock,
    pub location: TextBlock,
    pub icon: WeatherIcon,
    pub alignment: WeatherAlignment,
    pub icon_opacity: Option<u8>,
    pub horizontal_padding: i32,
    pub left_offset: i32,
    pub bottom_offset: i32,
    pub icon_size: i32,
    pub icon_gap: i32,
    pub location_gap: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LayoutRole {
    Hero,
    Auth,
    Footer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneModel {
    sections: Vec<SceneSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneSection {
    pub role: LayoutRole,
    pub widget: SceneWidget,
    pub gap_after: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SceneWidget {
    Clock(TextBlock),
    Date(TextBlock),
    Avatar,
    Username(TextBlock),
    Input(TextBlock),
    Status(TextBlock),
    Weather(SceneWeatherBlocks),
}

impl SceneModel {
    pub(super) fn sections_for_role(
        &self,
        role: LayoutRole,
    ) -> impl Iterator<Item = &SceneSection> {
        self.sections
            .iter()
            .filter(move |section| section.role == role)
    }

    pub(super) fn total_height_for_role(
        &self,
        role: LayoutRole,
        metrics: SceneMetrics,
        status: &ShellStatus,
    ) -> i32 {
        self.sections_for_role(role)
            .map(|section| section.height(metrics, status) + section.gap_after)
            .sum()
    }

    pub(super) fn anchor_height_for_role(
        &self,
        role: LayoutRole,
        metrics: SceneMetrics,
        status: &ShellStatus,
    ) -> i32 {
        let sections = self.sections_for_role(role).collect::<Vec<_>>();

        sections
            .iter()
            .enumerate()
            .filter(|(_, section)| !matches!(section.widget, SceneWidget::Status(_)))
            .map(|(index, section)| {
                let gap_after = if matches!(
                    sections.get(index + 1).map(|next| &next.widget),
                    Some(SceneWidget::Status(_))
                ) {
                    0
                } else {
                    section.gap_after
                };

                section.height(metrics, status) + gap_after
            })
            .sum()
    }
}

impl SceneSection {
    pub(super) fn new(role: LayoutRole, widget: SceneWidget, gap_after: i32) -> Self {
        Self {
            role,
            widget,
            gap_after,
        }
    }

    pub(super) fn height(&self, metrics: SceneMetrics, status: &ShellStatus) -> i32 {
        self.widget.height(metrics, status)
    }
}

impl SceneWidget {
    fn height(&self, metrics: SceneMetrics, _status: &ShellStatus) -> i32 {
        match self {
            Self::Clock(block)
            | Self::Date(block)
            | Self::Username(block)
            | Self::Status(block) => block.height as i32,
            Self::Avatar => metrics.avatar_size,
            Self::Input(_) => metrics.input_height,
            Self::Weather(blocks) => blocks.height(),
        }
    }
}

impl SceneWeatherBlocks {
    const DEFAULT_ICON_GAP: i32 = 8;
    const DEFAULT_LOCATION_GAP: i32 = 2;
    const MIN_ICON_SIZE: i32 = 18;
    const MAX_ICON_SIZE: i32 = 96;
    const MAX_GAP: i32 = 64;

    pub(super) fn height(&self) -> i32 {
        self.icon_size
            + self.icon_gap
            + self.temperature.height as i32
            + self.location_gap
            + self.location.height as i32
    }

    pub(super) fn clamped_icon_size(size: i32) -> i32 {
        size.clamp(Self::MIN_ICON_SIZE, Self::MAX_ICON_SIZE)
    }

    pub(super) fn clamped_icon_gap(size: i32) -> i32 {
        size.clamp(0, Self::MAX_GAP)
    }

    pub(super) fn clamped_location_gap(size: i32) -> i32 {
        size.clamp(0, Self::MAX_GAP)
    }

    pub(super) const fn default_icon_gap() -> i32 {
        Self::DEFAULT_ICON_GAP
    }

    pub(super) const fn default_location_gap() -> i32 {
        Self::DEFAULT_LOCATION_GAP
    }
}
