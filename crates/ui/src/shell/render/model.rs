use veila_renderer::text::TextBlock;

use super::{super::ShellStatus, layout::SceneMetrics, widgets::indicator_height};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SceneTextBlocks {
    pub clock: TextBlock,
    pub date: TextBlock,
    pub hint: TextBlock,
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
    Hint(TextBlock),
    Input,
    Indicator,
    Status(TextBlock),
}

impl SceneModel {
    pub(super) fn standard(blocks: SceneTextBlocks) -> Self {
        let mut sections = vec![
            SceneSection::new(LayoutRole::Hero, SceneWidget::Clock(blocks.clock), 4),
            SceneSection::new(LayoutRole::Hero, SceneWidget::Date(blocks.date), 30),
            SceneSection::new(LayoutRole::Hero, SceneWidget::Avatar, 18),
            SceneSection::new(LayoutRole::Auth, SceneWidget::Hint(blocks.hint), 26),
            SceneSection::new(LayoutRole::Auth, SceneWidget::Input, 12),
        ];

        let indicator_gap = if blocks.status.is_some() { 14 } else { 0 };
        sections.push(SceneSection::new(
            LayoutRole::Auth,
            SceneWidget::Indicator,
            indicator_gap,
        ));

        if let Some(status) = blocks.status {
            sections.push(SceneSection::new(
                LayoutRole::Auth,
                SceneWidget::Status(status),
                0,
            ));
        }

        Self { sections }
    }

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
}

impl SceneSection {
    fn new(role: LayoutRole, widget: SceneWidget, gap_after: i32) -> Self {
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
    fn height(&self, metrics: SceneMetrics, status: &ShellStatus) -> i32 {
        match self {
            Self::Clock(block) | Self::Date(block) | Self::Hint(block) | Self::Status(block) => {
                block.height as i32
            }
            Self::Avatar => metrics.avatar_size,
            Self::Input => metrics.input_height,
            Self::Indicator => indicator_height(status),
        }
    }
}

#[cfg(test)]
mod tests {
    use veila_renderer::{
        ClearColor,
        text::{TextBlock, TextStyle},
    };

    use super::{LayoutRole, SceneModel, SceneTextBlocks, SceneWidget};
    use crate::shell::{ShellStatus, render::layout::SceneMetrics};

    #[test]
    fn assigns_hero_and_auth_roles() {
        let model = SceneModel::standard(SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            hint: block("Type your password to unlock"),
            status: None,
        });

        let hero_sections = model
            .sections_for_role(LayoutRole::Hero)
            .collect::<Vec<_>>();
        let auth_sections = model
            .sections_for_role(LayoutRole::Auth)
            .collect::<Vec<_>>();

        assert_eq!(hero_sections.len(), 3);
        assert_eq!(auth_sections.len(), 3);
        assert!(matches!(hero_sections[0].widget, SceneWidget::Clock(_)));
        assert!(matches!(hero_sections[1].widget, SceneWidget::Date(_)));
        assert!(matches!(hero_sections[2].widget, SceneWidget::Avatar));
        assert!(matches!(auth_sections[0].widget, SceneWidget::Hint(_)));
        assert!(matches!(auth_sections[1].widget, SceneWidget::Input));
        assert!(matches!(auth_sections[2].widget, SceneWidget::Indicator));
    }

    #[test]
    fn appends_status_to_auth_role() {
        let with_status = SceneModel::standard(SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            hint: block("Type your password to unlock"),
            status: Some(block("Authentication failed")),
        });
        let without_status = SceneModel::standard(SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            hint: block("Type your password to unlock"),
            status: None,
        });

        let auth_sections = with_status
            .sections_for_role(LayoutRole::Auth)
            .collect::<Vec<_>>();

        assert!(matches!(
            auth_sections.last().expect("status section").widget,
            SceneWidget::Status(_)
        ));
        assert_eq!(
            with_status.total_height_for_role(
                LayoutRole::Auth,
                SceneMetrics::from_frame(1280, 720),
                &ShellStatus::Idle,
            ) - without_status.total_height_for_role(
                LayoutRole::Auth,
                SceneMetrics::from_frame(1280, 720),
                &ShellStatus::Idle,
            ),
            38
        );
    }

    #[test]
    fn footer_role_is_empty_in_default_model() {
        let model = SceneModel::standard(SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            hint: block("Type your password to unlock"),
            status: None,
        });

        assert_eq!(model.sections_for_role(LayoutRole::Footer).count(), 0);
    }

    fn block(text: &str) -> TextBlock {
        TextBlock {
            lines: vec![text.to_string()],
            style: TextStyle::new(ClearColor::opaque(255, 255, 255), 1),
            width: 100,
            height: 24,
        }
    }
}
