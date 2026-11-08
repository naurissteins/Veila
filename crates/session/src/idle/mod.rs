mod config;
mod lock;
mod logind;
mod options;
pub use options::IdleOptions;

use crate::{
    adapters::idle::IdleNotifier,
    record::{LockRecord, State},
};
use anyhow::Result;
use std::time::Duration;
use tokio::{
    signal::unix::{SignalKind, signal},
    sync::{mpsc, oneshot, watch},
};
use veila_common::IdleConfig;

enum Trigger {
    Lock,
    Sleep(oneshot::Sender<()>),
}

pub async fn run(options: IdleOptions) -> Result<()> {
    if options.help {
        println!(
            "Veila optional idle watcher\nUsage: veila idle [--config=PATH] [--session-id=ID]"
        );
        return Ok(());
    }
    let mut hangup = signal(SignalKind::hangup())?;
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    let _record = crate::process_status::WatcherRecord::acquire(options.config.clone())?;
    let (settings_tx, mut settings) = watch::channel(IdleConfig::default());
    let (reload, reload_rx) = mpsc::channel(1);
    let (triggers, mut requests) = mpsc::channel(8);
    let (events, mut lock_events) = mpsc::channel(8);
    let config_task = tokio::spawn(config::run(options.config.clone(), settings_tx, reload_rx));
    let logind_task = tokio::spawn(logind::run(settings.clone(), triggers, options.session_id));
    let mut idle = None;
    let mut idle_timeout = None;
    let mut child = false;
    let mut waiters: Vec<oneshot::Sender<()>> = Vec::new();
    let result = async {
        loop {
            tokio::select! {
                _ = terminate.recv() => break,
                _ = interrupt.recv() => break,
                _ = hangup.recv() => { let _ = reload.try_send(()); }
                changed = settings.changed() => {
                    changed?;
                    let timeout = settings.borrow_and_update().effective_lock_after_seconds().map(Duration::from_secs);
                    if timeout != idle_timeout {
                        idle_timeout = timeout;
                        idle = timeout.map(IdleNotifier::spawn);
                    }
                }
                () = idled(&mut idle) => {
                    request_lock(options.config.as_deref(), &events, &mut child);
                }
                request = requests.recv() => {
                    let Some(request) = request else { anyhow::bail!("idle logind worker stopped"); };
                    if let Trigger::Sleep(ack) = request {
                        waiters.retain(|ack| !ack.is_closed());
                        waiters.push(ack);
                    }
                    request_lock(options.config.as_deref(), &events, &mut child);
                    confirm(&mut waiters);
                }
                event = lock_events.recv(), if child => {
                    match event {
                        Some(lock::Event::Confirmed) => confirm(&mut waiters),
                        Some(lock::Event::Exited) => { child = false; confirm(&mut waiters); }
                        None => anyhow::bail!("lock reaper stopped"),
                    }
                }
                () = tokio::time::sleep(Duration::from_millis(50)), if !waiters.is_empty() => {
                    waiters.retain(|ack| !ack.is_closed());
                    confirm(&mut waiters);
                }
            }
        }
        Ok(())
    }.await;
    // Stopping this optional watcher must never terminate an acquired lock.
    config_task.abort();
    logind_task.abort();
    result
}

fn request_lock(
    path: Option<&std::path::Path>,
    events: &mpsc::Sender<lock::Event>,
    child: &mut bool,
) {
    if *child {
        return;
    }
    match LockRecord::active() {
        Ok(Some(State::Starting | State::Locked)) => return,
        Ok(Some(_)) => {}
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(%error, "could not inspect lock record; supervisor will validate ownership")
        }
    }
    match lock::spawn(path, events.clone()) {
        Ok(()) => {
            *child = true;
            tracing::info!("idle watcher spawned lock supervisor");
        }
        Err(error) => tracing::error!(%error, "idle watcher could not start lock"),
    }
}
fn confirm(waiters: &mut Vec<oneshot::Sender<()>>) {
    if !waiters.is_empty() && matches!(LockRecord::active(), Ok(Some(State::Locked))) {
        release(waiters);
    }
}
fn release(waiters: &mut Vec<oneshot::Sender<()>>) {
    for waiter in waiters.drain(..) {
        let _ = waiter.send(());
    }
}
async fn idled(idle: &mut Option<IdleNotifier>) {
    while let Some(notifier) = idle.as_mut() {
        if notifier.idled().await.is_some() {
            return;
        }
        *idle = None;
    }
    std::future::pending().await
}
