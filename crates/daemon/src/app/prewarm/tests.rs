use veila_common::AppConfig;
use veila_renderer::FrameSize;

use crate::app::output_probe::ProbedOutput;

use super::{PrewarmSize, generated_sizes, prewarm_inputs_changed, prewarm_jobs};

#[test]
fn detects_background_related_prewarm_changes() {
    let current = AppConfig::from_toml_str(
        r#"
                [background]
                mode = "file"
                path = "/tmp/one.jpg"
            "#,
    )
    .expect("current config");
    let next = AppConfig::from_toml_str(
        r#"
                [background]
                mode = "file"
                path = "/tmp/two.jpg"
            "#,
    )
    .expect("next config");

    assert!(prewarm_inputs_changed(&current, &next));
}

#[test]
fn ignores_unrelated_reload_changes_for_prewarm() {
    let current = AppConfig::from_toml_str(
        r##"
                [background]
                mode = "gradient"

                [visuals.clock]
                color = "#FFFFFF"
            "##,
    )
    .expect("current config");
    let next = AppConfig::from_toml_str(
        r##"
                [background]
                mode = "gradient"

                [visuals.clock]
                color = "#FF5353"
            "##,
    )
    .expect("next config");

    assert!(!prewarm_inputs_changed(&current, &next));
}

#[test]
fn detects_visual_layer_related_prewarm_changes() {
    let current = AppConfig::from_toml_str(
        r##"
                [background]
                mode = "gradient"

                [[visuals.layer]]
                kind = "icon"
                text = "one"
            "##,
    )
    .expect("current config");
    let next = AppConfig::from_toml_str(
        r##"
                [background]
                mode = "gradient"

                [[visuals.layer]]
                kind = "icon"
                text = "two"
            "##,
    )
    .expect("next config");

    assert!(prewarm_inputs_changed(&current, &next));
}

#[test]
fn prewarm_jobs_use_scaled_output_buffer_sizes() {
    let config = AppConfig::from_toml_str(
        r#"
                [background]
                mode = "file"
                path = "/tmp/wallpaper.jpg"
            "#,
    )
    .expect("config");
    let outputs = vec![ProbedOutput {
        name: Some(String::from("DP-1")),
        size: FrameSize::new(3840, 2160),
        scale: 2,
    }];

    let jobs = prewarm_jobs(&config.background, &outputs);

    assert_eq!(jobs.len(), 1);
    assert_eq!(
        jobs[0].sizes,
        vec![PrewarmSize {
            buffer: FrameSize::new(3840, 2160),
            scale: 2
        }]
    );
}

#[test]
fn generated_prewarm_sizes_keep_scale_with_buffer_size() {
    let config = AppConfig::from_toml_str(
        r#"
                [background]
                mode = "gradient"
            "#,
    )
    .expect("config");
    let outputs = vec![ProbedOutput {
        name: Some(String::from("DP-1")),
        size: FrameSize::new(3840, 2160),
        scale: 2,
    }];

    assert_eq!(
        generated_sizes(&config.background, &outputs),
        vec![PrewarmSize {
            buffer: FrameSize::new(3840, 2160),
            scale: 2
        }]
    );
}
