use std::process::Stdio;

use super::*;

const ACCEPT: &str = r#"{"Verdict":{"accepted":true,"message":null}}"#;

#[derive(Default)]
struct Recorder {
    notices: Vec<String>,
    challenges: Vec<Challenge>,
    answers: Vec<Option<&'static str>>,
    delay: Duration,
}

impl Conversation for Recorder {
    fn notice(&mut self, text: String) {
        self.notices.push(text);
    }

    fn challenge(&mut self, challenge: Challenge, _deadline: Instant) -> Option<Secret> {
        self.challenges.push(challenge);
        std::thread::sleep(self.delay);
        self.answers
            .remove(0)
            .map(|answer| Secret::from(answer.to_owned()))
    }
}

fn fake_helper(script: &str) -> Command {
    let mut command = Command::new("sh");
    command
        .args(["-c", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

fn short() -> Deadlines {
    Deadlines {
        initial: Duration::from_millis(200),
        interactive: Duration::from_millis(400),
        prompt: Duration::from_millis(100),
    }
}

fn attempt(
    script: &str,
    recorder: &mut Recorder,
    deadlines: Deadlines,
) -> Result<Verdict, ClientError> {
    run(
        fake_helper(script),
        "user",
        Secret::from(String::from("password")),
        recorder,
        deadlines,
    )
}

#[test]
fn accepts_verdict_after_start_request() {
    let script = format!(r#"read -r l; case "$l" in *'"Start"'*'"user"'*) echo '{ACCEPT}';; esac"#);
    let verdict =
        attempt(&script, &mut Recorder::default(), Deadlines::default()).expect("verdict");
    assert_eq!(
        verdict,
        Verdict {
            accepted: true,
            message: None
        }
    );
}

#[test]
fn reports_rejection_with_message() {
    let script =
        r#"read -r l; echo '{"Verdict":{"accepted":false,"message":"Authentication failure"}}'"#;
    let verdict = attempt(script, &mut Recorder::default(), Deadlines::default()).expect("verdict");
    assert!(!verdict.accepted);
    assert_eq!(verdict.message.as_deref(), Some("Authentication failure"));
}

#[test]
fn relays_notice_and_challenge_answer() {
    let script = format!(
        r#"read -r l; echo '{{"Notice":{{"text":"Touch key"}}}}'
echo '{{"Challenge":{{"sequence":1,"echo":false,"text":"Code:"}}}}'
read -r l; case "$l" in *'"Response"'*'"sequence":1'*'"123456"'*) echo '{ACCEPT}';; esac"#
    );
    let mut recorder = Recorder {
        answers: vec![Some("123456")],
        ..Recorder::default()
    };
    let verdict = attempt(&script, &mut recorder, Deadlines::default()).expect("verdict");
    assert!(verdict.accepted);
    assert_eq!(recorder.notices, ["Touch key"]);
    assert_eq!(
        recorder.challenges,
        [Challenge {
            sequence: 1,
            echo: false,
            text: String::from("Code:")
        }]
    );
}

#[test]
fn silent_helper_is_killed_at_deadline() {
    let started = Instant::now();
    let result = attempt("sleep 5", &mut Recorder::default(), short());
    assert!(matches!(result, Err(ClientError::Deadline)));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn crashed_helper_cannot_accept() {
    let result = attempt("read -r l; exit 1", &mut Recorder::default(), short());
    assert!(matches!(result, Err(ClientError::ClosedWithoutVerdict)));
}

#[test]
fn verdict_from_failing_helper_is_rejected() {
    let script = format!("read -r l; echo '{ACCEPT}'; exit 1");
    let result = attempt(&script, &mut Recorder::default(), short());
    assert!(matches!(result, Err(ClientError::HelperFailed)));
}

#[test]
fn oversized_frame_fails_attempt() {
    let script = "read -r l; head -c 5000 /dev/zero | tr '\\0' a; sleep 5";
    let result = attempt(script, &mut Recorder::default(), short());
    assert!(matches!(
        result,
        Err(ClientError::Protocol(ProtocolError::Oversize))
    ));
}

#[test]
fn truncated_frame_fails_attempt() {
    let result = attempt("read -r l; printf '{}'", &mut Recorder::default(), short());
    assert!(matches!(
        result,
        Err(ClientError::Protocol(ProtocolError::Truncated))
    ));
}

#[test]
fn stale_challenge_sequence_fails_attempt() {
    let script =
        r#"read -r l; echo '{"Challenge":{"sequence":2,"echo":false,"text":"Code:"}}'; sleep 5"#;
    let mut recorder = Recorder::default();
    let result = attempt(script, &mut recorder, short());
    assert!(matches!(result, Err(ClientError::InvalidChallenge)));
    assert!(recorder.challenges.is_empty());
}

#[test]
fn control_characters_in_challenge_fail_attempt() {
    let script = r#"read -r l; printf "%s\n" '{"Challenge":{"sequence":1,"echo":false,"text":"a\u001bb"}}'; sleep 5"#;
    let result = attempt(script, &mut Recorder::default(), short());
    assert!(matches!(result, Err(ClientError::InvalidChallenge)));
}

#[test]
fn cancelled_challenge_fails_attempt() {
    let script =
        r#"read -r l; echo '{"Challenge":{"sequence":1,"echo":false,"text":"Code:"}}'; sleep 5"#;
    let mut recorder = Recorder {
        answers: vec![None],
        ..Recorder::default()
    };
    let started = Instant::now();
    let result = attempt(script, &mut recorder, Deadlines::default());
    assert!(matches!(result, Err(ClientError::Cancelled)));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn late_challenge_answer_fails_attempt() {
    let script = r#"read -r l; echo '{"Challenge":{"sequence":1,"echo":false,"text":"Code:"}}'; read -r l; echo '{"Verdict":{"accepted":true,"message":null}}'"#;
    let mut recorder = Recorder {
        answers: vec![Some("123456")],
        delay: Duration::from_millis(150),
        ..Recorder::default()
    };
    let result = attempt(script, &mut recorder, short());
    assert!(matches!(result, Err(ClientError::ChallengeTimeout)));
}

#[test]
fn notice_flood_fails_attempt() {
    let script = r#"read -r l; i=0; while [ $i -le 32 ]; do echo '{"Notice":{"text":"x"}}'; i=$((i+1)); done; sleep 5"#;
    let mut recorder = Recorder::default();
    let result = attempt(script, &mut recorder, short());
    assert!(matches!(result, Err(ClientError::InvalidNotice)));
    assert_eq!(recorder.notices.len(), MAX_NOTICES as usize);
}
