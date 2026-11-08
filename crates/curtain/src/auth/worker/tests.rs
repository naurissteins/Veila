use super::Relay;
use crate::ipc::auth::{AuthEvent, ChallengeReply};
use calloop::channel::{Event, channel};
use std::time::{Duration, Instant};
use veila_auth::{Challenge, Conversation};
use veila_common::Secret;

fn challenge() -> Challenge {
    Challenge {
        sequence: 2,
        echo: true,
        text: "Token".into(),
    }
}

#[test]
fn relays_notice_and_challenge_with_attempt_identity() {
    let (sender, receiver) = channel();
    let (replies, incoming) = std::sync::mpsc::channel();
    let mut relay = Relay {
        attempt_id: 19,
        sender,
        replies: incoming,
    };
    relay.notice("Notice".into());
    replies
        .send(ChallengeReply::Response {
            sequence: 2,
            secret: Secret::from(String::from("answer")),
        })
        .expect("reply");
    assert_eq!(
        relay
            .challenge(challenge(), Instant::now() + Duration::from_secs(1))
            .expect("answer")
            .expose(),
        "answer"
    );
    let mut event_loop = calloop::EventLoop::try_new().expect("event loop");
    event_loop
        .handle()
        .insert_source(receiver, |event, _, events: &mut Vec<_>| {
            if let Event::Msg(event) = event {
                events.push(event);
            }
        })
        .expect("source");
    let mut events = Vec::new();
    event_loop
        .dispatch(Some(Duration::ZERO), &mut events)
        .expect("dispatch");
    assert_eq!(
        events,
        vec![
            AuthEvent::Notice {
                attempt_id: 19,
                text: "Notice".into()
            },
            AuthEvent::Challenge {
                attempt_id: 19,
                sequence: 2,
                echo: true,
                text: "Token".into()
            }
        ]
    );
}

#[test]
fn cancellation_stale_response_disconnect_and_deadline_do_not_supply_a_secret() {
    for reply in [
        Some(ChallengeReply::Cancel { sequence: 2 }),
        Some(ChallengeReply::Response {
            sequence: 1,
            secret: Secret::from(String::from("stale")),
        }),
        None,
    ] {
        let (sender, _receiver) = channel();
        let (replies, incoming) = std::sync::mpsc::channel();
        if let Some(reply) = reply {
            replies.send(reply).expect("reply");
        }
        drop(replies);
        let mut relay = Relay {
            attempt_id: 1,
            sender,
            replies: incoming,
        };
        assert!(
            relay
                .challenge(challenge(), Instant::now() + Duration::from_secs(1))
                .is_none()
        );
    }
    let (sender, _receiver) = channel();
    let (_replies, incoming) = std::sync::mpsc::channel();
    let mut relay = Relay {
        attempt_id: 1,
        sender,
        replies: incoming,
    };
    assert!(relay.challenge(challenge(), Instant::now()).is_none());
}
