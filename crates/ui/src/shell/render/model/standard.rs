use super::{LayoutRole, SceneModel, SceneSection, SceneTextBlocks, SceneWidget};

impl SceneModel {
    pub(crate) fn standard(
        blocks: SceneTextBlocks,
        clock_gap: Option<i32>,
        avatar_gap: Option<i32>,
        username_gap: Option<i32>,
        status_gap: Option<i32>,
    ) -> Self {
        let SceneTextBlocks {
            clock,
            date,
            username,
            placeholder,
            status,
        } = blocks;
        let clock_gap = clock_gap.unwrap_or(4).clamp(0, 48);
        let avatar_gap = avatar_gap.unwrap_or(10).clamp(0, 96);
        let username_gap = username_gap.unwrap_or(34).clamp(0, 96);
        let status_gap = status_gap.unwrap_or(14).clamp(0, 96);

        let mut sections = vec![
            SceneSection::new(LayoutRole::Hero, SceneWidget::Clock(clock), clock_gap),
            SceneSection::new(LayoutRole::Hero, SceneWidget::Date(date), 0),
            SceneSection::new(LayoutRole::Auth, SceneWidget::Avatar, avatar_gap),
        ];

        if let Some(username) = username {
            sections.push(SceneSection::new(
                LayoutRole::Auth,
                SceneWidget::Username(username),
                username_gap,
            ));
        }

        sections.push(SceneSection::new(
            LayoutRole::Auth,
            SceneWidget::Input(placeholder),
            if status.is_some() { status_gap } else { 0 },
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
}
