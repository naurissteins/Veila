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
    pub(super) fn standard(blocks: SceneTextBlocks) -> Self {
        let SceneTextBlocks {
            clock,
            date,
            username,
            placeholder,
            status,
        } = blocks;

        let mut sections = vec![
            SceneSection::new(LayoutRole::Hero, SceneWidget::Clock(clock), 4),
            SceneSection::new(LayoutRole::Hero, SceneWidget::Date(date), 0),
            SceneSection::new(LayoutRole::Auth, SceneWidget::Avatar, 10),
        ];

        if let Some(username) = username {
            sections.push(SceneSection::new(
                LayoutRole::Auth,
                SceneWidget::Username(username),
                34,
            ));
        }

        sections.push(SceneSection::new(
            LayoutRole::Auth,
            SceneWidget::Input(placeholder),
            if status.is_some() { 14 } else { 0 },
        ));

        if let Some(status) = status {
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
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
        });

        let hero_sections = model
            .sections_for_role(LayoutRole::Hero)
            .collect::<Vec<_>>();
        let auth_sections = model
            .sections_for_role(LayoutRole::Auth)
            .collect::<Vec<_>>();

        assert_eq!(hero_sections.len(), 2);
        assert_eq!(auth_sections.len(), 3);
        assert!(matches!(hero_sections[0].widget, SceneWidget::Clock(_)));
        assert!(matches!(hero_sections[1].widget, SceneWidget::Date(_)));
        assert!(matches!(auth_sections[0].widget, SceneWidget::Avatar));
        assert!(matches!(auth_sections[1].widget, SceneWidget::Username(_)));
        assert!(matches!(auth_sections[2].widget, SceneWidget::Input(_)));
    }

    #[test]
    fn appends_status_to_auth_role() {
        let with_status = SceneModel::standard(SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: Some(block("Authentication failed")),
        });
        let without_status = SceneModel::standard(SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
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
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
        });

        assert_eq!(model.sections_for_role(LayoutRole::Footer).count(), 0);
    }

    #[test]
    fn omits_username_widget_when_disabled() {
        let model = SceneModel::standard(SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: None,
            placeholder: block("Type your password to unlock"),
            status: None,
        });

        assert_eq!(model.sections_for_role(LayoutRole::Hero).count(), 2);
        assert!(
            model
                .sections_for_role(LayoutRole::Auth)
                .all(|section| !matches!(section.widget, SceneWidget::Username(_)))
        );
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
