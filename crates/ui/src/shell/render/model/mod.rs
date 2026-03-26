mod standard;
#[cfg(test)]
mod tests;

use veila_renderer::text::TextBlock;

use super::{super::ShellStatus, layout::SceneMetrics};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTextBlocks {
    pub clock: TextBlock,
    pub date: TextBlock,
    pub username: Option<TextBlock>,
    pub placeholder: TextBlock,
    pub status: Option<TextBlock>,
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
        }
    }
}
