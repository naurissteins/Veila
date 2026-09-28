use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    path::PathBuf,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use veila_common::Secret;
use veila_common::ipc::{ClientMessage, DaemonMessage, decode_message, encode_message};

use super::{AuthEvent, ChallengeReply, submit_password};
use calloop::channel::{Channel, Event, channel};

const RECV_TIMEOUT: Duration = Duration::from_secs(5);

fn receive_event(receiver: Channel<AuthEvent>) -> AuthEvent {
    let mut event_loop = calloop::EventLoop::<Option<AuthEvent>>::try_new().expect("event loop");
    event_loop
        .handle()
        .insert_source(receiver, |event, _, received| {
            if let Event::Msg(event) = event {
                *received = Some(event);
            }
        })
        .expect("auth event source");
    let mut received = None;
    event_loop
        .dispatch(Some(RECV_TIMEOUT), &mut received)
        .expect("auth event dispatch");
    received.expect("auth event before timeout")
}

#[test]
fn relays_two_ordered_challenges_on_one_connection() {
    let path = unique_socket_path("auth-challenges");
    let listener = UnixListener::bind(&path).expect("bind auth socket");
    let daemon = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut line = String::new();
        reader.read_line(&mut line).expect("start request");
        assert!(matches!(
            decode_message::<ClientMessage>(&line),
            Ok(ClientMessage::SubmitPassword { attempt_id: 17, .. })
        ));
        for (sequence, echo, answer) in [(1, false, "123456"), (2, true, "yes")] {
            let challenge = encode_message(&DaemonMessage::AuthenticationChallenge {
                attempt_id: 17,
                sequence,
                echo,
                text: format!("Prompt {sequence}"),
            })
            .expect("encode challenge");
            stream
                .write_all(format!("{challenge}\n").as_bytes())
                .expect("write challenge");
            line.clear();
            reader.read_line(&mut line).expect("read answer");
            assert!(matches!(
                decode_message::<ClientMessage>(&line),
                Ok(ClientMessage::AuthenticationResponse { attempt_id: 17, sequence: id, secret })
                    if id == sequence && secret.expose() == answer
            ));
        }
        let verdict = encode_message(&DaemonMessage::AuthenticationRejected {
            attempt_id: 17,
            retry_after_ms: None,
            failed_attempts: Some(1),
            message: None,
        })
        .expect("encode verdict");
        stream
            .write_all(format!("{verdict}\n").as_bytes())
            .expect("write verdict");
    });
    let (sender, receiver) = channel();
    let replies = submit_password(
        path.clone(),
        17,
        Secret::from(String::from("password")),
        sender,
    );
    let mut event_loop = calloop::EventLoop::<Vec<AuthEvent>>::try_new().expect("event loop");
    event_loop
        .handle()
        .insert_source(receiver, |event, _, events| {
            if let Event::Msg(event) = event {
                events.push(event);
            }
        })
        .expect("auth event source");
    let mut events = Vec::new();
    for (sequence, echo, answer) in [(1, false, "123456"), (2, true, "yes")] {
        for _ in 0..2 {
            if events.len() >= sequence as usize {
                break;
            }
            event_loop
                .dispatch(Some(RECV_TIMEOUT), &mut events)
                .expect("dispatch");
        }
        assert!(
            events.len() >= sequence as usize,
            "challenge before timeout"
        );
        assert!(matches!(&events[sequence as usize - 1],
            AuthEvent::Challenge { attempt_id: 17, sequence: id, echo: visible, .. }
                if *id == sequence && *visible == echo
        ));
        replies
            .send(ChallengeReply::Response {
                sequence,
                secret: Secret::from(answer.to_owned()),
            })
            .expect("send response");
    }
    for _ in 0..2 {
        if events.len() >= 3 {
            break;
        }
        event_loop
            .dispatch(Some(RECV_TIMEOUT), &mut events)
            .expect("dispatch");
    }
    assert!(events.len() >= 3, "verdict before timeout");
    assert!(matches!(
        events[2],
        AuthEvent::Rejected { attempt_id: 17, .. }
    ));
    daemon.join().expect("daemon fixture");
    std::fs::remove_file(path).ok();
}

#[test]
fn rejects_skipped_challenge_sequence() {
    let path = unique_socket_path("auth-skipped-challenge");
    let listener = UnixListener::bind(&path).expect("bind auth socket");
    let daemon = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request = String::new();
        BufReader::new(stream.try_clone().expect("clone stream"))
            .read_line(&mut request)
            .expect("start request");
        let challenge = encode_message(&DaemonMessage::AuthenticationChallenge {
            attempt_id: 19,
            sequence: 2,
            echo: false,
            text: String::from("Code"),
        })
        .expect("encode challenge");
        stream
            .write_all(format!("{challenge}\n").as_bytes())
            .expect("write challenge");
    });
    let (sender, receiver) = channel();
    let _replies = submit_password(
        path.clone(),
        19,
        Secret::from(String::from("password")),
        sender,
    );
    assert_eq!(
        receive_event(receiver),
        AuthEvent::Failed { attempt_id: 19 }
    );
    daemon.join().expect("daemon fixture");
    std::fs::remove_file(path).ok();
}

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

#[test]
fn reports_failure_when_daemon_socket_is_missing() {
    let (sender, receiver) = channel();

    submit_password(
        unique_socket_path("auth-missing"),
        3,
        Secret::from(String::from("secret")),
        sender,
    );

    assert_eq!(receive_event(receiver), AuthEvent::Failed { attempt_id: 3 });
}

#[test]
fn reports_failure_when_daemon_closes_without_verdict() {
    let path = unique_socket_path("auth-silent");
    let listener = UnixListener::bind(&path).expect("bind auth socket");
    let daemon = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream);
        let mut request = String::new();
        let _ = reader.read_line(&mut request);
    });

    let (sender, receiver) = channel();
    submit_password(
        path.clone(),
        5,
        Secret::from(String::from("secret")),
        sender,
    );

    assert_eq!(receive_event(receiver), AuthEvent::Failed { attempt_id: 5 });

    daemon.join().expect("daemon stub");
    std::fs::remove_file(&path).ok();
}

#[test]
fn forwards_daemon_rejection() {
    let path = unique_socket_path("auth-rejected");
    let listener = UnixListener::bind(&path).expect("bind auth socket");
    let daemon = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request = String::new();
        reader.read_line(&mut request).expect("read request");

        let response = encode_message(&DaemonMessage::AuthenticationRejected {
            attempt_id: 9,
            retry_after_ms: Some(250),
            failed_attempts: Some(2),
            message: None,
        })
        .expect("encode response");
        stream
            .write_all(format!("{response}\n").as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });

    let (sender, receiver) = channel();
    submit_password(
        path.clone(),
        9,
        Secret::from(String::from("secret")),
        sender,
    );

    assert_eq!(
        receive_event(receiver),
        AuthEvent::Rejected {
            attempt_id: 9,
            retry_after_ms: Some(250),
            failed_attempts: Some(2),
            message: None,
        }
    );

    daemon.join().expect("daemon stub");
    std::fs::remove_file(&path).ok();
}

#[test]
fn reports_failure_when_daemon_rejects_the_request_format() {
    let path = unique_socket_path("auth-error-response");
    let listener = UnixListener::bind(&path).expect("bind auth socket");
    let daemon = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request = String::new();
        reader.read_line(&mut request).expect("read request");

        let response = encode_message(&DaemonMessage::Error {
            reason: "unsupported request".to_string(),
        })
        .expect("encode response");
        stream
            .write_all(format!("{response}\n").as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });

    let (sender, receiver) = channel();
    submit_password(
        path.clone(),
        11,
        Secret::from(String::from("secret")),
        sender,
    );

    assert_eq!(
        receive_event(receiver),
        AuthEvent::Failed { attempt_id: 11 }
    );

    daemon.join().expect("daemon stub");
    std::fs::remove_file(&path).ok();
}
