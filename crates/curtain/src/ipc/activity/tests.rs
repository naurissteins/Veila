use std::{
    fs,
    io::{BufRead, BufReader},
    os::unix::net::UnixListener,
    sync::mpsc::{self, TrySendError},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use nix::sys::socket::{Backlog, listen};
use veila_common::ipc::{ClientMessage, decode_message};

use super::{ActivityNotifier, run_worker, send_activity};

const INTERVAL: Duration = super::NOTIFICATION_INTERVAL;
const TEST_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn sends_a_leading_and_trailing_update_for_a_burst() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (updates, observed) = mpsc::channel();
    let worker = thread::spawn(move || {
        run_worker(receiver, INTERVAL, || {
            updates.send(Instant::now()).expect("observer")
        })
    });
    sender.try_send(()).expect("first input");
    let first = observed.recv_timeout(TEST_TIMEOUT).expect("leading update");
    let mut last_input = first;
    for _ in 0..10_000 {
        last_input = Instant::now();
        let _ = sender.try_send(());
    }
    let trailing = observed
        .recv_timeout(TEST_TIMEOUT)
        .expect("trailing update");
    assert!(trailing.duration_since(first) >= INTERVAL);
    assert!(
        trailing >= last_input,
        "final input must reset the daemon timer"
    );
    assert!(
        observed.recv_timeout(INTERVAL * 2).is_err(),
        "idle worker must stop sending"
    );
    drop(sender);
    worker.join().expect("worker");
}

#[test]
fn sustained_input_is_rate_limited_and_retains_its_tail() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (updates, observed) = mpsc::channel();
    let worker = thread::spawn(move || {
        run_worker(receiver, INTERVAL, || {
            updates.send(Instant::now()).expect("observer")
        })
    });
    sender.try_send(()).expect("first input");
    let first = observed.recv_timeout(TEST_TIMEOUT).expect("leading update");
    let mut last_input = first;
    while first.elapsed() < INTERVAL * 3 {
        last_input = Instant::now();
        let _ = sender.try_send(());
        thread::sleep(Duration::from_millis(5));
    }
    thread::sleep(INTERVAL * 2);
    let times: Vec<_> = std::iter::once(first).chain(observed.try_iter()).collect();
    assert!(times.len() >= 2 && times.len() <= 5, "updates: {times:?}");
    assert!(
        times
            .windows(2)
            .all(|pair| pair[1].duration_since(pair[0]) >= INTERVAL)
    );
    assert!(*times.last().expect("updates") >= last_input);
    drop(sender);
    worker.join().expect("worker");
}

#[test]
fn slow_ipc_keeps_the_input_queue_bounded_and_nonblocking() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (started, observed) = mpsc::channel();
    let (resume, gate) = mpsc::channel();
    let worker = thread::spawn(move || {
        run_worker(receiver, INTERVAL, || {
            started.send(()).expect("observer");
            gate.recv().expect("release callback");
        })
    });
    sender.try_send(()).expect("first input");
    observed.recv_timeout(TEST_TIMEOUT).expect("IPC started");
    sender.try_send(()).expect("one pending event");
    for _ in 0..10_000 {
        assert!(matches!(sender.try_send(()), Err(TrySendError::Full(()))));
    }
    drop(sender);
    resume.send(()).expect("release IPC");
    worker.join().expect("worker");
}

#[test]
fn disconnected_idle_worker_exits_without_notifications() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let (done, observed) = mpsc::channel();
    let worker = thread::spawn(move || {
        run_worker(receiver, INTERVAL, || panic!("no activity was submitted"));
        done.send(()).expect("observer");
    });
    drop(sender);
    observed
        .recv_timeout(TEST_TIMEOUT)
        .expect("worker shutdown");
    worker.join().expect("worker");
}

#[test]
fn standalone_notifier_never_starts_a_worker() {
    let mut notifier = ActivityNotifier::new(None);
    notifier.notify();
    assert!(notifier.sender.is_none());
}

fn socket_path(label: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "veila-activity-{label}-{}-{stamp}.sock",
        std::process::id()
    ))
}

fn receive_activity(listener: &UnixListener) {
    let (stream, _) = listener.accept().expect("connection");
    stream
        .set_read_timeout(Some(TEST_TIMEOUT))
        .expect("timeout");
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .expect("payload");
    assert_eq!(
        decode_message::<ClientMessage>(line.trim_end()).expect("valid IPC"),
        ClientMessage::Activity
    );
}

#[test]
fn sends_valid_activity_and_reconnects_after_socket_replacement() {
    let path = socket_path("reconnect");
    assert!(
        send_activity(&path).is_err(),
        "missing daemon must not stall"
    );
    let listener = UnixListener::bind(&path).expect("listener");
    send_activity(&path).expect("activity");
    receive_activity(&listener);
    drop(listener);
    fs::remove_file(&path).expect("old socket");
    let replacement = UnixListener::bind(&path).expect("replacement listener");
    send_activity(&path).expect("activity after daemon recovery");
    receive_activity(&replacement);
    drop(replacement);
    fs::remove_file(path).expect("socket cleanup");
}

#[test]
fn full_accept_backlog_does_not_block_activity_delivery() {
    let path = socket_path("backlog");
    let listener = UnixListener::bind(&path).expect("listener");
    listen(&listener, Backlog::new(1).expect("backlog")).expect("small backlog");
    let clients: Vec<_> = (0..2)
        .map(|_| std::os::unix::net::UnixStream::connect(&path).expect("fill backlog"))
        .collect();
    let (result, observed) = mpsc::channel();
    let target = path.clone();
    let worker = thread::spawn(move || {
        result
            .send(send_activity(&target).is_err())
            .expect("observer")
    });
    assert!(
        observed
            .recv_timeout(TEST_TIMEOUT)
            .expect("nonblocking failure")
    );
    worker.join().expect("worker");
    drop(clients);
    drop(listener);
    fs::remove_file(path).expect("socket cleanup");
}
