use std::ffi::{OsStr, OsString};

use anyhow::{Context, Result, anyhow};
use nonstick::{
    AuthnFlags, ConversationAdapter, Result as PamResult, Transaction, TransactionBuilder,
};
use veila_common::Secret;

fn pam_service() -> String {
    if let Ok(service) = std::env::var("VEILA_PAM_SERVICE") {
        return service;
    }

    if std::path::Path::new("/etc/pam.d/veila").exists() {
        return String::from("veila");
    }

    String::from("system-auth")
}

struct PasswordConversation {
    username: String,
    password: Secret,
}

impl ConversationAdapter for PasswordConversation {
    fn prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        Ok(OsString::from(&self.username))
    }

    fn masked_prompt(&self, _request: impl AsRef<OsStr>) -> PamResult<OsString> {
        // PAM takes ownership of this copy, so it is out of our reach from here on
        Ok(OsString::from(self.password.expose()))
    }

    fn error_msg(&self, _message: impl AsRef<OsStr>) {}

    fn info_msg(&self, _message: impl AsRef<OsStr>) {}
}

pub fn authenticate(username: &str, password: &Secret) -> Result<()> {
    let service = pam_service();
    let conversation = PasswordConversation {
        username: username.to_string(),
        password: password.clone(),
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
