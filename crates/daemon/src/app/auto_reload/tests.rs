use std::{
    fs,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tokio::{io::AsyncBufReadExt, net::UnixListener};
use veila_common::AppConfig;

use super::{AppRuntime, AutoReloadTrigger, handle};
use crate::{
    app::{runtime::Fixture, suspend::SuspendDecision},
    domain::{
        auth::{AuthAdmission, AuthState},
        lock_state::LockState,
    },
};

const TRIGGERS: [AutoReloadTrigger; 4] = [
    AutoReloadTrigger::Config,
    AutoReloadTrigger::Theme,
    AutoReloadTrigger::Include,
    AutoReloadTrigger::Wallpaper,
];

fn config(enabled: bool, delay: u64) -> String {
    format!(
        "[background]\nmode = 'solid'\n[lock]\nauto_reload_config = {enabled}\nauth_backoff_base_ms = {delay}\n[weather]\nenabled = false\n[battery]\nenabled = false\n[now_playing]\nenabled = false\n"
    )
}

fn runtime(fixture: &Fixture, enabled: bool) -> AppRuntime {
    let path = fixture.root.join("config.toml");
    fs::write(&path, config(enabled, 1000)).expect("config");
    AppRuntime::new(AppConfig::load(Some(&path)).expect("load"), 0, 0)
}

async fn reload(fixture: &Fixture, runtime: &mut AppRuntime, trigger: AutoReloadTrigger) {
    handle(trigger, Some(&fixture.root.join("config.toml")), runtime).await;
}

fn assert_timestamp(runtime: &AppRuntime, before: u128) {
    let after = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let recorded = u128::from(runtime.last_reload_unix_ms.expect("reload timestamp"));
    assert!((before..=after).contains(&recorded));
}

#[tokio::test]
async fn every_trigger_applies_config_and_records_success() {
    for trigger in TRIGGERS {
        let fixture = Fixture::new();
        let mut runtime = runtime(&fixture, true);
        runtime.auth_state.finish_failure(Instant::now());
        fs::write(fixture.root.join("config.toml"), config(true, 2000)).expect("edit");
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis();
        reload(&fixture, &mut runtime, trigger).await;
        assert_eq!(runtime.loaded_config.config.lock.auth_backoff_base_ms, 2000);
        assert_eq!(
            runtime.last_reload_result,
            Some(format!("ok:{}", trigger.source()))
        );
        assert_timestamp(&runtime, before);
        assert_eq!(runtime.auth_state.failed_attempts(), 0);
        runtime.auth_state.finish_failure(Instant::now());
        assert!(
            matches!(runtime.auth_state.admit(Instant::now()), AuthAdmission::RateLimited(delay) if delay > Duration::from_millis(1900))
        );
    }
}

#[tokio::test]
async fn every_trigger_records_load_failure_without_changing_policy() {
    for trigger in TRIGGERS {
        let fixture = Fixture::new();
        let mut runtime = runtime(&fixture, true);
        fs::write(fixture.root.join("config.toml"), "[lock\n").expect("invalid edit");
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis();
        reload(&fixture, &mut runtime, trigger).await;
        let prefix = format!(
            "error:{}:failed to auto reload daemon config after {}:",
            trigger.source(),
            trigger.change_description()
        );
        assert!(
            runtime
                .last_reload_result
                .as_ref()
                .expect("failure")
                .starts_with(&prefix)
        );
        assert_timestamp(&runtime, before);
        assert_eq!(runtime.loaded_config.config.lock.auth_backoff_base_ms, 1000);
    }
}

#[tokio::test]
async fn disabled_config_edits_and_parse_errors_preserve_previous_status() {
    let fixture = Fixture::new();
    let mut runtime = runtime(&fixture, false);
    runtime.last_reload_result = Some("ok:manual".into());
    runtime.last_reload_unix_ms = Some(123);
    for text in [config(false, 2000), "[lock\n".into()] {
        fs::write(fixture.root.join("config.toml"), text).expect("edit");
        reload(&fixture, &mut runtime, AutoReloadTrigger::Config).await;
        assert_eq!(runtime.loaded_config.config.lock.auth_backoff_base_ms, 1000);
        assert_eq!(runtime.last_reload_result.as_deref(), Some("ok:manual"));
        assert_eq!(runtime.last_reload_unix_ms, Some(123));
    }
}

#[tokio::test]
async fn config_edits_can_enable_and_disable_auto_reload() {
    let fixture = Fixture::new();
    let mut runtime = runtime(&fixture, false);
    for enabled in [true, false, true] {
        fs::write(fixture.root.join("config.toml"), config(enabled, 2000)).expect("edit");
        reload(&fixture, &mut runtime, AutoReloadTrigger::Config).await;
        assert_eq!(
            runtime.loaded_config.config.lock.auto_reload_config,
            enabled
        );
        assert_eq!(
            runtime.last_reload_result.as_deref(),
            Some("ok:config-change")
        );
    }
}

#[tokio::test]
async fn pending_non_config_triggers_keep_their_policy_after_auto_reload_is_disabled() {
    for trigger in [
        AutoReloadTrigger::Theme,
        AutoReloadTrigger::Include,
        AutoReloadTrigger::Wallpaper,
    ] {
        let fixture = Fixture::new();
        let mut runtime = runtime(&fixture, false);
        fs::write(fixture.root.join("config.toml"), config(false, 2000)).expect("edit");
        reload(&fixture, &mut runtime, trigger).await;
        assert_eq!(runtime.loaded_config.config.lock.auth_backoff_base_ms, 2000);
        assert!(!runtime.loaded_config.config.lock.auto_reload_config);
        assert_eq!(
            runtime.last_reload_result,
            Some(format!("ok:{}", trigger.source()))
        );
    }
}

#[tokio::test]
async fn live_reload_preserves_backoff_in_flight_auth_and_suspend_deadline() {
    for trigger in TRIGGERS {
        let mut fixture = Fixture::new();
        let mut runtime = runtime(&fixture, true);
        let path = fixture
            .active
            .as_ref()
            .expect("active")
            .control_socket_path
            .clone();
        fs::remove_file(&path).expect("remove fixture socket");
        let listener = UnixListener::bind(&path).expect("control listener");
        let received = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("connection");
            let mut line = String::new();
            tokio::io::BufReader::new(stream)
                .read_line(&mut line)
                .await
                .expect("read");
            line
        });
        runtime.active = fixture.active.take();
        runtime.state = LockState::Locked;
        let now = Instant::now();
        runtime.auth_state = AuthState::after_daemon_recovery(runtime.auth_policy, now);
        runtime.auth_state.start_attempt();
        runtime
            .suspend_state
            .set_policy(Some(Duration::from_secs(10)), false, false, now, true);
        runtime.suspend_state.arm(now - Duration::from_secs(20));
        let text = config(true, 2000).replace("[weather]", "suspend_seconds = 10\nsuspend_only_on_battery = false\nskip_suspend_while_media_playing = false\n[weather]");
        fs::write(fixture.root.join("config.toml"), text).expect("edit");
        reload(&fixture, &mut runtime, trigger).await;
        let line = tokio::time::timeout(Duration::from_secs(3), received)
            .await
            .expect("timeout")
            .expect("task");
        assert_eq!(line, "\"ReloadConfig\"\n");
        assert_eq!(runtime.state, LockState::Locked);
        assert!(runtime.active.is_some());
        assert_eq!(
            runtime.last_reload_result,
            Some(format!("ok:{}", trigger.source()))
        );
        assert!(runtime.auth_state.in_flight());
        runtime.auth_state.finish_failure(now);
        assert!(matches!(
            runtime.auth_state.admit(now + Duration::from_secs(11)),
            AuthAdmission::RateLimited(_)
        ));
        assert_eq!(
            runtime
                .suspend_state
                .evaluate(now, true, false, None, false),
            SuspendDecision::Ready
        );
    }
}

#[tokio::test]
async fn failed_live_reload_keeps_active_resources_and_records_error_for_every_trigger() {
    for trigger in TRIGGERS {
        let mut fixture = Fixture::new();
        let mut runtime = runtime(&fixture, true);
        runtime.active = fixture.active.take();
        runtime.state = LockState::Locked;
        runtime.auth_state = AuthState::after_daemon_recovery(runtime.auth_policy, Instant::now());
        fs::write(fixture.root.join("config.toml"), config(true, 2000)).expect("edit");
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_millis();
        reload(&fixture, &mut runtime, trigger).await;
        assert!(
            runtime
                .last_reload_result
                .as_ref()
                .expect("error")
                .starts_with(&format!(
                    "error:{}:failed to forward live config reload to curtain:",
                    trigger.source()
                ))
        );
        assert_timestamp(&runtime, before);
        assert_eq!(runtime.loaded_config.config.lock.auth_backoff_base_ms, 2000);
        assert_eq!(runtime.state, LockState::Locked);
        assert!(runtime.active.is_some());
        assert!(matches!(
            runtime.auth_state.admit(Instant::now()),
            AuthAdmission::RateLimited(_)
        ));
    }
}

#[tokio::test]
async fn unresolved_active_lock_does_not_become_unlocked_during_reload() {
    let fixture = Fixture::new();
    let mut runtime = runtime(&fixture, true);
    runtime.state = LockState::Locking;
    reload(&fixture, &mut runtime, AutoReloadTrigger::Config).await;
    assert_eq!(
        runtime.last_reload_result.as_deref(),
        Some(
            "error:config-change:failed to forward live config reload to curtain: active lock has no control socket"
        )
    );
    assert_eq!(runtime.state, LockState::Locking);
    assert!(runtime.active.is_none());
}
