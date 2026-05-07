use veila_common::InputAlignment;

use super::{
    LayoutRole, SceneModel, SceneSection, SceneTextBlocks, SceneWidget, StandardSceneConfig,
};

const DEFAULT_STATUS_GAP: i32 = 14;

impl SceneModel {
    pub(crate) fn standard(blocks: SceneTextBlocks, config: StandardSceneConfig) -> Self {
        let SceneTextBlocks {
            clock,
            date,
            username,
            placeholder,
            status,
            weather,
        } = blocks;
        let StandardSceneConfig {
            identity_visible,
            input_visible,
            input_alignment,
            avatar_enabled,
            clock_gap,
            avatar_gap,
            username_gap,
        } = config;
        let clock_gap = clock_gap.unwrap_or(4).clamp(0, 48);
        let avatar_gap = avatar_gap.unwrap_or(10).clamp(0, 96);
        let username_gap = username_gap.unwrap_or(34).clamp(0, 96);
        let status_gap = DEFAULT_STATUS_GAP;

        let mut sections = Vec::new();

        if let Some(clock) = clock {
            sections.push(SceneSection::new(
                LayoutRole::Hero,
                SceneWidget::Clock(clock),
                if date.is_some() { clock_gap } else { 0 },
            ));
        }

        if let Some(date) = date {
            sections.push(SceneSection::new(
                LayoutRole::Hero,
                SceneWidget::Date(date),
                0,
            ));
        }

        if identity_visible {
            if avatar_enabled {
                sections.push(SceneSection::new(
                    LayoutRole::Auth,
                    SceneWidget::Avatar,
                    avatar_gap,
                ));
            }

            if let Some(username) = username {
                sections.push(SceneSection::new(
                    LayoutRole::Auth,
                    SceneWidget::Username(username),
                    username_gap,
                ));
            }
        }

        if input_visible {
            let status_before_input = matches!(
                input_alignment,
                InputAlignment::BottomCenter
                    | InputAlignment::BottomRight
                    | InputAlignment::BottomLeft
            );

            if let Some(status) = status {
                if status_before_input {
                    sections.push(SceneSection::new(
                        LayoutRole::Auth,
                        SceneWidget::Status(status),
                        status_gap,
                    ));
                    sections.push(SceneSection::new(
                        LayoutRole::Auth,
                        SceneWidget::Input(placeholder),
                        0,
                    ));
                } else {
                    sections.push(SceneSection::new(
                        LayoutRole::Auth,
                        SceneWidget::Input(placeholder),
                        status_gap,
                    ));
                    sections.push(SceneSection::new(
                        LayoutRole::Auth,
                        SceneWidget::Status(status),
                        0,
                    ));
                }
            } else {
                sections.push(SceneSection::new(
                    LayoutRole::Auth,
                    SceneWidget::Input(placeholder),
                    0,
                ));
            }
        } else if let Some(status) = status {
            sections.push(SceneSection::new(
                LayoutRole::Auth,
                SceneWidget::Status(status),
                0,
            ));
        }

        if let Some(weather) = weather {
            sections.push(SceneSection::new(
                LayoutRole::Footer,
                SceneWidget::Weather(weather),
                0,
            ));
        }

        Self { sections }
    }
}
