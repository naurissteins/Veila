use crate::{adapters::process, app::runtime::AuthResult};
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
};
use veila_session::services::fingerprint::{FingerprintHandle as SharedHandle, FingerprintMatch};

pub(super) struct FingerprintHandle(SharedHandle);
impl FingerprintHandle {
    pub(super) fn new() -> Self {
        Self(SharedHandle::new())
    }
    pub(super) fn should_discard_auth_result(&self, result: &AuthResult) -> bool {
        matches!(result, AuthResult::Succeeded { attempt_id, .. } if self.0.should_discard_attempt(*attempt_id))
    }
    pub(super) async fn forward_status_updates(&mut self, path: Option<&PathBuf>) {
        while let Some(status) = self.0.take_status() {
            if let Some(path) = path
                && let Err(error) =
                    process::request_curtain_fingerprint_status_update(path, status.as_ref()).await
            {
                tracing::warn!("failed to forward fingerprint status to curtain: {error:#}");
            }
        }
    }
}
impl Deref for FingerprintHandle {
    type Target = SharedHandle;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for FingerprintHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<FingerprintMatch> for AuthResult {
    fn from(value: FingerprintMatch) -> Self {
        Self::Succeeded {
            attempt_id: value.attempt_id,
            started_at: value.started_at,
            elapsed_ms: value.elapsed_ms,
        }
    }
}
