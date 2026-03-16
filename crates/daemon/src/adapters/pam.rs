use std::ffi::{OsStr, OsString};

use anyhow::{Context, Result, anyhow};
use nonstick::{
    AuthnFlags, ConversationAdapter, Result as PamResult, Transaction, TransactionBuilder,
};

fn pam_service() -> String {
    if let Ok(service) = std::env::var("KWYLOCK_PAM_SERVICE") {
        return service;
    }

    if std::path::Path::new("/etc/pam.d/kwylock").exists() {
        return String::from("kwylock");
    }

    String::from("system-auth")
}

struct PasswordConversation {
    username: String,
    password: String,
}

impl ConversationAdapter for PasswordConversation {
    fn prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(OsString::from(&self.username))
    }

    fn masked_prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(OsString::from(&self.password))
    }

    fn error_msg(&self, _message: impl AsRef<OsStr>) {}

    fn info_msg(&self, _message: impl AsRef<OsStr>) {}
}

pub fn authenticate(username: &str, password: &str) -> Result<()> {
    let service = pam_service();
    let conversation = PasswordConversation {
        username: username.to_string(),
        password: password.to_string(),
    };

    let mut transaction = TransactionBuilder::new_with_service(&service)
        .username(username)
        .build(conversation.into_conversation())
        .with_context(|| format!("failed to initialize PAM transaction for service {service}"))?;

    transaction
        .authenticate(AuthnFlags::empty())
        .map_err(|_| anyhow!("PAM authentication failed"))?;

    Ok(())
}
