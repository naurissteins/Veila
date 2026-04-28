use veila_renderer::{
    ClearColor,
    text::{TextBlock, TextStyle},
};

use veila_common::{ClockStyle, InputAlignment};

use super::{AuthGroup, LayoutRole, SceneClockBlocks, SceneModel, SceneTextBlocks, SceneWidget};
use crate::shell::{ShellStatus, render::layout::SceneMetrics};

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
        InputAlignment::CenterCenter,
        true,
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
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Authentication failed")),
            weather: None,
        },
        InputAlignment::CenterCenter,
        true,
        None,
        None,
        None,
        None,
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
        InputAlignment::CenterCenter,
        true,
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
            SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter),
            &ShellStatus::Idle,
        ) - without_status.total_height_for_role(
            LayoutRole::Auth,
            SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter),
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
        InputAlignment::CenterCenter,
        true,
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
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: None,
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        InputAlignment::CenterCenter,
        true,
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
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        InputAlignment::CenterCenter,
        true,
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
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        InputAlignment::CenterCenter,
        true,
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
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Authentication failed")),
            weather: None,
        },
        InputAlignment::CenterCenter,
        true,
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
fn places_status_above_input_for_bottom_aligned_layouts() {
    let model = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: Some(block("Checking authentication")),
            weather: None,
        },
        InputAlignment::BottomCenter,
        true,
        None,
        None,
        None,
        Some(20),
    );

    let auth_sections = model
        .sections_for_role(LayoutRole::Auth)
        .collect::<Vec<_>>();

    assert!(matches!(auth_sections[2].widget, SceneWidget::Status(_)));
    assert_eq!(auth_sections[2].gap_after, 20);
    assert!(matches!(auth_sections[3].widget, SceneWidget::Input(_)));
}

#[test]
fn keeps_auth_anchor_height_stable_when_status_is_added() {
    let metrics =
        SceneMetrics::from_frame(1280, 720, None, None, None, InputAlignment::CenterCenter);
    let without_status = SceneModel::standard(
        SceneTextBlocks {
            clock: Some(clock_blocks("09:05")),
            date: Some(block("Tuesday, March 24")),
            username: Some(block("ramces")),
            placeholder: Some(block("Type your password to unlock")),
            status: None,
            weather: None,
        },
        InputAlignment::CenterCenter,
        true,
        None,
        None,
        None,
        Some(20),
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
        InputAlignment::CenterCenter,
        true,
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
        InputAlignment::CenterCenter,
        true,
        None,
        None,
        None,
        Some(20),
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
        InputAlignment::CenterCenter,
        true,
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
        InputAlignment::CenterCenter,
        false,
        None,
        None,
        None,
        None,
    );

    assert!(
        model
            .sections_for_role(LayoutRole::Auth)
            .all(|section| !matches!(section.widget, SceneWidget::Avatar))
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
