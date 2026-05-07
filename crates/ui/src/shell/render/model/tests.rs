use veila_renderer::{
    ClearColor,
    text::{TextBlock, TextStyle},
};

use veila_common::ClockStyle;

use super::{
    AuthGroup, LayoutRole, SceneClockBlocks, SceneModel, SceneTextBlocks, SceneWidget,
    StandardSceneConfig,
};
use crate::shell::{ShellStatus, render::layout::SceneMetrics};

fn standard_scene_config() -> StandardSceneConfig {
    StandardSceneConfig {
        identity_visible: true,
        input_visible: true,
        avatar_enabled: true,
        clock_gap: None,
        avatar_gap: None,
        username_gap: None,
    }
}

#[test]
fn assigns_hero_and_auth_roles() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        standard_scene_config(),
    );

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
    let with_status = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Authentication failed")),
            weather: None,
        },
        standard_scene_config(),
    );
    let without_status = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        standard_scene_config(),
    );

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
            SceneMetrics::from_frame(1280, 720, None, None, None),
            &ShellStatus::Idle,
        ) - without_status.total_height_for_role(
            LayoutRole::Auth,
            SceneMetrics::from_frame(1280, 720, None, None, None),
            &ShellStatus::Idle,
        ),
        38
    );
}

#[test]
fn footer_role_is_empty_in_default_model() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        standard_scene_config(),
    );

    assert_eq!(model.sections_for_role(LayoutRole::Footer).count(), 0);
}

#[test]
fn omits_username_widget_when_disabled() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: None,
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        standard_scene_config(),
    );

    assert_eq!(model.sections_for_role(LayoutRole::Hero).count(), 2);
    assert!(
        model
            .sections_for_role(LayoutRole::Auth)
            .all(|section| !matches!(section.widget, SceneWidget::Username(_)))
    );
}

#[test]
fn uses_configured_username_gap() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        StandardSceneConfig {
            username_gap: Some(24),
            ..standard_scene_config()
        },
    );

    let auth_sections = model
        .sections_for_role(LayoutRole::Auth)
        .collect::<Vec<_>>();

    assert_eq!(auth_sections[1].gap_after, 24);
}

#[test]
fn uses_configured_avatar_gap() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        StandardSceneConfig {
            avatar_gap: Some(18),
            ..standard_scene_config()
        },
    );

    let auth_sections = model
        .sections_for_role(LayoutRole::Auth)
        .collect::<Vec<_>>();

    assert_eq!(auth_sections[0].gap_after, 18);
}

#[test]
fn uses_fixed_status_gap() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Authentication failed")),
            weather: None,
        },
        standard_scene_config(),
    );

    let auth_sections = model
        .sections_for_role(LayoutRole::Auth)
        .collect::<Vec<_>>();

    assert_eq!(auth_sections[2].gap_after, 14);
}

#[test]
fn keeps_auth_anchor_height_stable_when_status_is_added() {
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);
    let without_status = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        standard_scene_config(),
    );
    let with_status = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Authentication failed")),
            weather: None,
        },
        standard_scene_config(),
    );

    assert_eq!(
        without_status.anchor_height_for_role(LayoutRole::Auth, metrics, &ShellStatus::Idle),
        with_status.anchor_height_for_role(LayoutRole::Auth, metrics, &ShellStatus::Idle),
    );
}

#[test]
fn splits_auth_sections_into_identity_and_input_groups() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Checking authentication")),
            weather: None,
        },
        standard_scene_config(),
    );

    let identity_sections = model
        .sections_for_auth_group(AuthGroup::Identity)
        .collect::<Vec<_>>();
    let input_sections = model
        .sections_for_auth_group(AuthGroup::Input)
        .collect::<Vec<_>>();

    assert_eq!(identity_sections.len(), 2);
    assert!(matches!(identity_sections[0].widget, SceneWidget::Avatar));
    assert!(matches!(
        identity_sections[1].widget,
        SceneWidget::Username(_)
    ));
    assert_eq!(input_sections.len(), 2);
    assert!(matches!(input_sections[0].widget, SceneWidget::Input(_)));
    assert!(matches!(input_sections[1].widget, SceneWidget::Status(_)));
}

#[test]
fn uses_configured_clock_gap() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        StandardSceneConfig {
            clock_gap: Some(12),
            ..standard_scene_config()
        },
    );

    let hero_sections = model
        .sections_for_role(LayoutRole::Hero)
        .collect::<Vec<_>>();

    assert_eq!(hero_sections[0].gap_after, 12);
}

#[test]
fn omits_avatar_widget_when_disabled() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        StandardSceneConfig {
            avatar_enabled: false,
            ..standard_scene_config()
        },
    );

    assert!(
        model
            .sections_for_role(LayoutRole::Auth)
            .all(|section| !matches!(section.widget, SceneWidget::Avatar))
    );
}

#[test]
fn keeps_identity_sections_when_only_input_group_is_hidden() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Checking authentication")),
            weather: None,
        },
        StandardSceneConfig {
            input_visible: false,
            ..standard_scene_config()
        },
    );

    let auth_sections = model
        .sections_for_role(LayoutRole::Auth)
        .collect::<Vec<_>>();

    assert_eq!(auth_sections.len(), 3);
    assert!(matches!(auth_sections[0].widget, SceneWidget::Avatar));
    assert!(matches!(auth_sections[1].widget, SceneWidget::Username(_)));
    assert!(matches!(auth_sections[2].widget, SceneWidget::Status(_)));
}

#[test]
fn renders_hidden_hint_when_full_auth_stack_is_hidden() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Checking authentication")),
            weather: None,
        },
        StandardSceneConfig {
            identity_visible: false,
            input_visible: false,
            ..standard_scene_config()
        },
    );

    assert_eq!(model.sections_for_role(LayoutRole::Hero).count(), 2);
    let auth_sections = model
        .sections_for_role(LayoutRole::Auth)
        .collect::<Vec<_>>();
    assert_eq!(auth_sections.len(), 1);
    assert!(matches!(auth_sections[0].widget, SceneWidget::Status(_)));
}

#[test]
fn hidden_hint_contributes_to_auth_anchor_height_when_it_is_the_only_auth_widget() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Press any key or click to continue")),
            weather: None,
        },
        StandardSceneConfig {
            identity_visible: false,
            input_visible: false,
            ..standard_scene_config()
        },
    );
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);

    assert_eq!(
        model.anchor_height_for_role(LayoutRole::Auth, metrics, &ShellStatus::Idle),
        24
    );
    assert_eq!(
        model.anchor_height_for_auth_group(AuthGroup::Input, metrics, &ShellStatus::Idle),
        24
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

fn clock_blocks(text: &str) -> SceneClockBlocks {
    SceneClockBlocks {
        style: ClockStyle::Standard,
        primary: block(text),
        secondary: None,
        meridiem: None,
        meridiem_offset_x: 0,
        meridiem_offset_y: 0,
    }
}
