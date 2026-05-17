use super::{FrameBackendKind, FrameBackendPreference, fallback_reason};

#[test]
fn parses_backend_preference_values() {
    assert_eq!(
        FrameBackendPreference::from_env_value("software"),
        Some(FrameBackendPreference::Software)
    );
    assert_eq!(
        FrameBackendPreference::from_env_value("shm"),
        Some(FrameBackendPreference::Software)
    );
    assert_eq!(
        FrameBackendPreference::from_env_value("gpu"),
        Some(FrameBackendPreference::Gpu)
    );
    assert_eq!(
        FrameBackendPreference::from_env_value("auto"),
        Some(FrameBackendPreference::Auto)
    );
    assert_eq!(FrameBackendPreference::from_env_value("banana"), None);
}

#[test]
fn names_backend_kinds_for_logs() {
    assert_eq!(FrameBackendKind::Software.as_str(), "software");
    assert_eq!(FrameBackendKind::Gpu.as_str(), "gpu");
}

#[test]
fn auto_without_compiled_gpu_stays_quiet() {
    assert_eq!(
        fallback_reason(FrameBackendPreference::Auto, super::GPU_NOT_COMPILED_REASON),
        None
    );
}

#[test]
fn requested_gpu_reports_fallback_reason() {
    assert_eq!(
        fallback_reason(FrameBackendPreference::Gpu, super::GPU_NOT_COMPILED_REASON),
        Some(super::GPU_NOT_COMPILED_REASON)
    );
}
