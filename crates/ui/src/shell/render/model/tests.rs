use veila_renderer::{
    ClearColor,
    text::{TextBlock, TextStyle},
};

use super::{LayoutRole, SceneModel, SceneTextBlocks, SceneWidget};
use crate::shell::{ShellStatus, render::layout::SceneMetrics};

#[test]
fn assigns_hero_and_auth_roles() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
            weather: None,
        },
        None,
        None,
        None,
        None,
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
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: Some(block("Authentication failed")),
            weather: None,
        },
        None,
        None,
        None,
        None,
    );
    let without_status = SceneModel::standard(
        SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
            weather: None,
        },
        None,
        None,
        None,
        None,
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
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
            weather: None,
        },
        None,
        None,
        None,
        None,
    );

    assert_eq!(model.sections_for_role(LayoutRole::Footer).count(), 0);
}

#[test]
fn omits_username_widget_when_disabled() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: None,
            placeholder: block("Type your password to unlock"),
            status: None,
            weather: None,
        },
        None,
        None,
        None,
        None,
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
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
            weather: None,
        },
        None,
        None,
        Some(24),
        None,
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
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
            weather: None,
        },
        None,
        Some(18),
        None,
        None,
    );

    let auth_sections = model
        .sections_for_role(LayoutRole::Auth)
        .collect::<Vec<_>>();

    assert_eq!(auth_sections[0].gap_after, 18);
}

#[test]
fn uses_configured_status_gap() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: Some(block("Authentication failed")),
            weather: None,
        },
        None,
        None,
        None,
        Some(20),
    );

    let auth_sections = model
        .sections_for_role(LayoutRole::Auth)
        .collect::<Vec<_>>();

    assert_eq!(auth_sections[2].gap_after, 20);
}

#[test]
fn keeps_auth_anchor_height_stable_when_status_is_added() {
    let metrics = SceneMetrics::from_frame(1280, 720, None, None, None);
    let without_status = SceneModel::standard(
        SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
            weather: None,
        },
        None,
        None,
        None,
        Some(20),
    );
    let with_status = SceneModel::standard(
        SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: Some(block("Authentication failed")),
            weather: None,
        },
        None,
        None,
        None,
        Some(20),
    );

    assert_eq!(
        without_status.anchor_height_for_role(LayoutRole::Auth, metrics, &ShellStatus::Idle),
        with_status.anchor_height_for_role(LayoutRole::Auth, metrics, &ShellStatus::Idle),
    );
}

#[test]
fn uses_configured_clock_gap() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: block("09:05"),
            date: block("Tuesday, March 24"),
            username: Some(block("ramces")),
            placeholder: block("Type your password to unlock"),
            status: None,
            weather: None,
        },
        Some(12),
        None,
        None,
        None,
    );

    let hero_sections = model
        .sections_for_role(LayoutRole::Hero)
        .collect::<Vec<_>>();

    assert_eq!(hero_sections[0].gap_after, 12);
}

fn block(text: &str) -> TextBlock {
    TextBlock {
        lines: vec![text.to_string()],
        style: TextStyle::new(ClearColor::opaque(255, 255, 255), 1),
        width: 100,
        height: 24,
    }
}
