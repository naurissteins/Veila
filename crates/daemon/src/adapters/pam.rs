use std::{
    ffi::{OsStr, OsString},
    io::{Read, Write},
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use nonstick::{
    AuthnFlags, ConversationAdapter, ErrorCode, Result as PamResult, Transaction,
    TransactionBuilder,
};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::timeout,
};
use veila_common::{Secret, ipc::encode_secret_message};
use zeroize::Zeroizing;

use super::process::{PAM_HELPER_PROCESS_NAME, PAM_HELPER_SUBCOMMAND};

const AUTH_DEADLINE: Duration = Duration::from_secs(30);
const MAX_HELPER_MESSAGE: usize = 4096;
const MAX_PAM_TEXT_CHARS: usize = 160;

#[derive(Serialize, Deserialize)]
struct PamRequest {
    username: String,
    secret: Secret,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PamReply {
    pub accepted: bool,
    pub message: Option<String>,
}

struct PasswordConversation {
    username: String,
    password: Secret,
    password_used: AtomicBool,
    message: Arc<Mutex<Option<String>>>,
}

impl ConversationAdapter for PasswordConversation {
    fn prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(OsString::from(&self.username))
    }

    fn masked_prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        // a second hidden challenge needs new user input, never reuse the password as a factor
        if self.password_used.swap(true, Ordering::Relaxed) {
            return Err(ErrorCode::ConversationError);
        }
        Ok(OsString::from(self.password.expose()))
    }

    fn error_msg(&self, message: impl AsRef<OsStr>) {
        save_message(&self.message, message.as_ref());
    }

    fn info_msg(&self, message: impl AsRef<OsStr>) {
        save_message(&self.message, message.as_ref());
    }
}

fn save_message(slot: &Mutex<Option<String>>, message: &OsStr) {
    let clean: String = message
        .to_string_lossy()
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_PAM_TEXT_CHARS)
        .collect();
    if !clean.is_empty()
        && let Ok(mut slot) = slot.lock()
    {
        *slot = Some(clean);
    }
}

fn pam_service() -> String {
    #[cfg(debug_assertions)]
    if let Ok(service) = std::env::var("VEILA_PAM_SERVICE") {
        tracing::warn!(service, "using debug PAM service override");
        return service;
    }

    if std::path::Path::new("/etc/pam.d/veila").exists() {
        return String::from("veila");
    }
    String::from("system-auth")
}

fn authenticate_local(request: PamRequest) -> PamReply {
    let service = pam_service();
    let message = Arc::new(Mutex::new(None));
    let conversation = PasswordConversation {
        username: request.username.clone(),
        password: request.secret,
        password_used: AtomicBool::new(false),
        message: Arc::clone(&message),
    };
    let result = TransactionBuilder::new_with_service(&service)
        .username(&request.username)
        .build(conversation.into_conversation())
        .and_then(|mut transaction| transaction.authenticate(AuthnFlags::empty()));
    let text = message.lock().ok().and_then(|slot| slot.clone());
    PamReply {
        accepted: result.is_ok(),
        message: text,
    }
}

pub fn run_helper() -> Result<()> {
    let mut input = Zeroizing::new(Vec::new());
    std::io::stdin()
        .take((MAX_HELPER_MESSAGE + 1) as u64)
        .read_to_end(&mut input)
        .context("failed to read PAM helper request")?;
    if input.len() > MAX_HELPER_MESSAGE {
        bail!("PAM helper request exceeded limit");
    }
    let request: PamRequest =
        serde_json::from_slice(&input).context("invalid PAM helper request")?;
    drop(input);
    let reply = authenticate_local(request);
    serde_json::to_writer(std::io::stdout().lock(), &reply)
        .context("failed to write PAM helper reply")?;
    std::io::stdout()
        .flush()
        .context("failed to flush PAM helper reply")
}

pub async fn authenticate(username: &str, secret: Secret) -> Result<PamReply> {
    let request = PamRequest {
        username: username.to_owned(),
        secret,
    };
    let payload = encode_secret_message(&request).context("failed to encode PAM helper request")?;
    let mut child = Command::new("/proc/self/exe")
        .arg0(PAM_HELPER_PROCESS_NAME)
        .arg(PAM_HELPER_SUBCOMMAND)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("failed to start PAM helper")?;
    run_with_deadline(&mut child, &payload, AUTH_DEADLINE).await
}

async fn run_with_deadline(
    child: &mut tokio::process::Child,
    payload: &[u8],
    deadline: Duration,
) -> Result<PamReply> {
    let result = timeout(deadline, exchange(child, payload)).await;
    match result {
        Ok(reply) => reply,
        Err(_) => {
            let _ = child.start_kill();
            let _ = timeout(Duration::from_millis(250), child.wait()).await;
            bail!("PAM helper exceeded authentication deadline")
        }
    }
}

async fn exchange(child: &mut tokio::process::Child, payload: &[u8]) -> Result<PamReply> {
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("PAM helper stdin missing"))?;
    stdin
        .write_all(payload)
        .await
        .context("failed to send PAM request")?;
    drop(stdin);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("PAM helper stdout missing"))?;
    let mut response = Vec::new();
    stdout
        .take((MAX_HELPER_MESSAGE + 1) as u64)
        .read_to_end(&mut response)
        .await
        .context("failed to read PAM reply")?;
    let status = child.wait().await.context("failed to reap PAM helper")?;
    if !status.success() || response.len() > MAX_HELPER_MESSAGE {
        bail!("PAM helper failed or returned an oversized reply");
    }
    serde_json::from_slice(&response).context("invalid PAM helper reply")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_masked_prompt_does_not_reuse_password() {
        let conversation = PasswordConversation {
            username: String::from("test"),
            password: Secret::from(String::from("secret")),
            password_used: AtomicBool::new(false),
            message: Arc::new(Mutex::new(None)),
        };
        assert!(conversation.masked_prompt("Password:").is_ok());
        assert!(conversation.masked_prompt("One-time code:").is_err());
    }

    #[test]
    fn pam_message_is_bounded_and_strips_controls() {
        let message = Mutex::new(None);
        save_message(&message, OsStr::new("Locked\naccount"));
        assert_eq!(
            message.lock().ok().and_then(|slot| slot.clone()).as_deref(),
            Some("Lockedaccount")
        );
    }

    #[tokio::test]
    async fn hung_helper_is_killed_at_deadline() {
        let mut child = Command::new("sleep")
            .arg("5")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn fixture");
        let result = run_with_deadline(&mut child, b"{}", Duration::from_millis(30)).await;
        assert!(result.is_err());
        assert!(child.try_wait().expect("inspect fixture").is_some());
    }

    #[tokio::test]
    async fn crashed_helper_cannot_accept_authentication() {
        let mut child = Command::new("false")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn fixture");
        let result = run_with_deadline(&mut child, b"{}", Duration::from_secs(1)).await;
        assert!(result.is_err());
    }
}
