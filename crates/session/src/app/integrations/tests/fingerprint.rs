use super::*;
use crate::services::fingerprint::authorization::Authority;

#[test]
fn fingerprint_verdict_requires_current_generation_revision_config_and_live_curtain() {
    for invalid in [
        "generation",
        "revision",
        "disabled",
        "unconfirmed",
        "revoked",
        "exited",
        "none",
        "valid",
    ] {
        let mut h = Harness::new();
        h.running.config.fingerprint.enabled = true;
        h.running.child = Some(
            std::process::Command::new("sleep")
                .arg("30")
                .spawn()
                .unwrap(),
        );
        let authority = Authority::default();
        let authorization = authority.issue();
        let mut generation = 2;
        let mut revision = 0;
        match invalid {
            "generation" => generation = 1,
            "revision" => revision = 1,
            "disabled" => h.running.config.fingerprint.enabled = false,
            "unconfirmed" => h.running.locked = false,
            "revoked" => authority.revoke(),
            "exited" => {
                let child = h.running.child.as_mut().unwrap();
                child.kill().unwrap();
                child.wait().unwrap();
            }
            "none" => {
                let mut child = h.running.child.take().unwrap();
                child.kill().unwrap();
                child.wait().unwrap();
            }
            _ => {}
        }
        h.events
            .try_send(Event::FingerprintMatch {
                generation,
                revision,
                attempt_id: 1 << 63,
                authorization,
            })
            .unwrap_or_else(|_| panic!("event queue"));
        h.running.read_integrations();
        let frame = frame::read::<SupervisorMessage>(&mut std::io::BufReader::new(&mut h.curtain));
        if invalid == "valid" {
            assert_eq!(
                frame.unwrap(),
                Some(SupervisorMessage::Control(CurtainControlMessage::Unlock {
                    attempt_id: Some(1 << 63)
                }))
            );
        } else {
            assert!(
                frame.is_err(),
                "{invalid} fingerprint verdict escaped validation"
            );
        }
    }
}
