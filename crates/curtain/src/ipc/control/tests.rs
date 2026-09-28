use std::{
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    sync::mpsc::channel,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use nix::unistd::Uid;
use veila_common::ipc::{
    CurtainControlMessage, CurtainControlResponse, CurtainLockState, decode_message, encode_message,
};

use super::{ControlEvent, ControlSender, run_listener};

const RECV_TIMEOUT: Duration = Duration::from_secs(5);

fn unique_socket_path(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "veila-test-{label}-{}-{stamp}.sock",
        std::process::id()
    ))
}

fn send_payload(path: &Path, payload: &str) {
    let mut stream = UnixStream::connect(path).expect("connect to control socket");
    stream.write_all(payload.as_bytes()).expect("write payload");
    stream.flush().expect("flush payload");
}

fn encoded_unlock(attempt_id: u64) -> String {
    let encoded = encode_message(&CurtainControlMessage::Unlock {
        attempt_id: Some(attempt_id),
    })
    .expect("encode unlock");
    format!("{encoded}\n")
}

#[test]
fn control_sender_wakes_calloop() {
    let (sender, receiver) = channel();
    let (ping, source) = calloop::ping::make_ping().expect("control wake source");
    let mut event_loop = calloop::EventLoop::<bool>::try_new().expect("event loop");
    event_loop
        .handle()
        .insert_source(source, |(), _, woke| *woke = true)
        .expect("ping source");
    let sender = ControlSender::new(sender, ping);
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        sender.send(ControlEvent::Reload).expect("send reload");
    });

    let mut woke = false;
    event_loop
        .dispatch(Some(RECV_TIMEOUT), &mut woke)
        .expect("control dispatch");

    assert!(woke);
    assert_eq!(receiver.try_recv(), Ok(ControlEvent::Reload));
}

#[test]
fn delivers_unlock_after_malformed_connections() {
    let path = unique_socket_path("control-resilience");
    let listener = UnixListener::bind(&path).expect("bind control socket");
    let (sender, receiver) = channel();
    let (ping, _source) = calloop::ping::make_ping().expect("control wake source");
    let handle = thread::spawn({
        let owner_uid = Uid::effective().as_raw();
        move || {
            run_listener(
                listener,
                owner_uid,
                ControlSender::new(sender, ping),
                Arc::new(AtomicU8::new(0)),
            )
        }
    });

    // each of these previously killed the listener thread for the rest of the lock session
    send_payload(&path, "not json at all\n");
    send_payload(&path, "{\"Unlock\": \n");
    send_payload(&path, "truncated without newline");
    UnixStream::connect(&path).expect("connect and close without sending");

    send_payload(&path, &encoded_unlock(7));

    let event = receiver
        .recv_timeout(RECV_TIMEOUT)
        .expect("unlock must still be delivered after malformed connections");
    assert_eq!(
        event,
        ControlEvent::Unlock {
            attempt_id: Some(7)
        }
    );

    handle.join().expect("listener thread should exit cleanly");
    std::fs::remove_file(&path).ok();
}

#[test]
fn keeps_serving_updates_before_unlock() {
    let path = unique_socket_path("control-continuity");
    let listener = UnixListener::bind(&path).expect("bind control socket");
    let (sender, receiver) = channel();
    let (ping, _source) = calloop::ping::make_ping().expect("control wake source");
    let handle = thread::spawn({
        let owner_uid = Uid::effective().as_raw();
        move || {
            run_listener(
                listener,
                owner_uid,
                ControlSender::new(sender, ping),
                Arc::new(AtomicU8::new(0)),
            )
        }
    });

    let reload = encode_message(&CurtainControlMessage::ReloadConfig).expect("encode reload");
    send_payload(&path, &format!("{reload}\n"));
    assert_eq!(
        receiver.recv_timeout(RECV_TIMEOUT).expect("reload event"),
        ControlEvent::Reload
    );

    send_payload(&path, "}{ still not json\n");
    send_payload(&path, &encoded_unlock(1));

    assert_eq!(
        receiver.recv_timeout(RECV_TIMEOUT).expect("unlock event"),
        ControlEvent::Unlock {
            attempt_id: Some(1)
        }
    );

    handle.join().expect("listener thread should exit cleanly");
    std::fs::remove_file(&path).ok();
}

#[test]
fn stops_when_curtain_receiver_is_gone() {
    let path = unique_socket_path("control-shutdown");
    let listener = UnixListener::bind(&path).expect("bind control socket");
    let (sender, receiver) = channel::<ControlEvent>();
    let (ping, _source) = calloop::ping::make_ping().expect("control wake source");
    let handle = thread::spawn({
        let owner_uid = Uid::effective().as_raw();
        move || {
            run_listener(
                listener,
                owner_uid,
                ControlSender::new(sender, ping),
                Arc::new(AtomicU8::new(0)),
            )
        }
    });

    drop(receiver);
    let reload = encode_message(&CurtainControlMessage::ReloadConfig).expect("encode reload");
    send_payload(&path, &format!("{reload}\n"));

    handle
        .join()
        .expect("listener thread should stop once the receiver is dropped");
    std::fs::remove_file(&path).ok();
}

#[test]
fn probe_reports_compositor_lock_without_consuming_listener() {
    let path = unique_socket_path("control-probe");
    let listener = UnixListener::bind(&path).expect("bind control socket");
    let (sender, receiver) = channel();
    let (ping, _source) = calloop::ping::make_ping().expect("control wake source");
    let state = Arc::new(AtomicU8::new(0));
    let listener_state = state.clone();
    let handle = thread::spawn(move || {
        run_listener(
            listener,
            Uid::effective().as_raw(),
            ControlSender::new(sender, ping),
            listener_state,
        )
    });
    for expected in [CurtainLockState::Starting, CurtainLockState::Locked] {
        if expected == CurtainLockState::Locked {
            state.store(1, Ordering::Release);
        }
        let mut stream = UnixStream::connect(&path).expect("connect probe");
        stream.write_all(b"\"Probe\"\n").expect("send probe");
        stream
            .set_read_timeout(Some(RECV_TIMEOUT))
            .expect("set timeout");
        let mut reply = String::new();
        stream.read_to_string(&mut reply).expect("read probe reply");
        let response: CurtainControlResponse =
            decode_message(reply.trim_end()).expect("decode probe");
        assert_eq!(response, CurtainControlResponse::Status { state: expected });
    }
    send_payload(&path, &encoded_unlock(2));
    assert_eq!(
        receiver.recv_timeout(RECV_TIMEOUT).expect("unlock"),
        ControlEvent::Unlock {
            attempt_id: Some(2)
        }
    );
    handle.join().expect("listener exit");
    std::fs::remove_file(&path).expect("remove control socket");
}
