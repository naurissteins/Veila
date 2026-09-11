use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::sync::{mpsc::UnboundedSender, watch};
use veila_common::FingerprintStatus;
use zbus::{proxy, zvariant::OwnedObjectPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyOutcome {
    Matched,
    NotMatched,
    NoEnrolledFingers,
    Unavailable,
    Cancelled,
}

#[proxy(
    interface = "net.reactivated.Fprint.Manager",
    default_service = "net.reactivated.Fprint",
    default_path = "/net/reactivated/Fprint/Manager"
)]
trait FprintManager {
    fn get_default_device(&self) -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "net.reactivated.Fprint.Device",
    default_service = "net.reactivated.Fprint"
)]
trait FprintDevice {
    fn claim(&self, username: &str) -> zbus::Result<()>;
    fn release(&self) -> zbus::Result<()>;
    fn list_enrolled_fingers(&self, username: &str) -> zbus::Result<Vec<String>>;
    fn verify_start(&self, finger_name: &str) -> zbus::Result<()>;
    fn verify_stop(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn verify_status(&self, result: &str, done: bool) -> zbus::Result<()>;
}

pub(crate) async fn verify_once(
    username: &str,
    status_sender: &UnboundedSender<Option<FingerprintStatus>>,
    cancel: &mut watch::Receiver<bool>,
) -> Result<VerifyOutcome> {
    if cancellation_requested(cancel) {
        return Ok(VerifyOutcome::Cancelled);
    }

    let connection = zbus::Connection::system()
        .await
        .context("failed to connect to system D-Bus for fprintd")?;
    let manager = FprintManagerProxy::new(&connection)
        .await
        .context("failed to create fprintd manager proxy")?;
    let device_path = match manager.get_default_device().await {
        Ok(path) => path,
        Err(error) => {
            tracing::debug!("fprintd default device unavailable: {error}");
            let _ = status_sender.send(Some(FingerprintStatus::Unavailable));
            return Ok(VerifyOutcome::Unavailable);
        }
    };
    let device = FprintDeviceProxy::builder(&connection)
        .path(device_path.as_str())
        .context("invalid fprintd device object path")?
        .build()
        .await
        .context("failed to create fprintd device proxy")?;

    if let Err(error) = device.claim(username).await {
        tracing::warn!("failed to claim fprintd device: {error}");
        let _ = status_sender.send(Some(FingerprintStatus::Unavailable));
        return Ok(VerifyOutcome::Unavailable);
    }

    let result = if cancellation_requested(cancel) {
        Ok(VerifyOutcome::Cancelled)
    } else {
        verify_claimed_device(username, &device, status_sender, cancel).await
    };
    if let Err(error) = device.release().await {
        tracing::debug!("failed to release fprintd device: {error}");
    }
    result
}

async fn verify_claimed_device(
    username: &str,
    device: &FprintDeviceProxy<'_>,
    status_sender: &UnboundedSender<Option<FingerprintStatus>>,
    cancel: &mut watch::Receiver<bool>,
) -> Result<VerifyOutcome> {
    let enrolled = device
        .list_enrolled_fingers(username)
        .await
        .context("failed to list enrolled fingerprints")?;
    if enrolled.is_empty() {
        let _ = status_sender.send(Some(FingerprintStatus::NoEnrolledFingers));
        return Ok(VerifyOutcome::NoEnrolledFingers);
    }

    let mut stream = device
        .receive_verify_status()
        .await
        .context("failed to subscribe to fprintd verification status")?;
    let _ = status_sender.send(Some(FingerprintStatus::Ready));
    if cancellation_requested(cancel) {
        return Ok(VerifyOutcome::Cancelled);
    }
    device
        .verify_start("any")
        .await
        .context("failed to start fprintd verification")?;

    loop {
        let signal = tokio::select! {
            biased;
            _ = wait_for_cancellation(cancel) => {
                stop_verification(device, "cancelled").await;
                return Ok(VerifyOutcome::Cancelled);
            }
            signal = stream.next() => signal,
        };
        let Some(signal) = signal else {
            stop_verification(device, "disconnected").await;
            return Ok(VerifyOutcome::Unavailable);
        };
        let args = match signal.args() {
            Ok(args) => args,
            Err(error) => {
                stop_verification(device, "invalid-status").await;
                return Err(error).context("failed to decode fprintd verification status");
            }
        };
        let status = verify_status(args.result(), *args.done());
        let _ = status_sender.send(Some(status));
        if !*args.done() {
            continue;
        }

        let outcome = if matches!(status, FingerprintStatus::Accepted) {
            VerifyOutcome::Matched
        } else {
            VerifyOutcome::NotMatched
        };
        stop_verification(device, "completed").await;
        return Ok(outcome);
    }
}

async fn stop_verification(device: &FprintDeviceProxy<'_>, reason: &str) {
    if let Err(error) = device.verify_stop().await {
        tracing::debug!(reason, "failed to stop fprintd verification: {error}");
    }
}

fn cancellation_requested(cancel: &watch::Receiver<bool>) -> bool {
    *cancel.borrow()
}

async fn wait_for_cancellation(cancel: &mut watch::Receiver<bool>) {
    if cancellation_requested(cancel) {
        return;
    }

    while cancel.changed().await.is_ok() {
        if cancellation_requested(cancel) {
            return;
        }
    }
}

pub(crate) fn verify_status(result: &str, done: bool) -> FingerprintStatus {
    match (result, done) {
        ("verify-match", true) => FingerprintStatus::Accepted,
        ("verify-no-match", true) => FingerprintStatus::NotRecognized,
        (
            "verify-swipe-too-short" | "verify-finger-not-centered" | "verify-remove-and-retry",
            _,
        ) => FingerprintStatus::NotRecognized,
        ("verify-disconnected", _) => FingerprintStatus::Unavailable,
        (_, false) => FingerprintStatus::Scanning,
        _ => FingerprintStatus::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::verify_status;
    use veila_common::FingerprintStatus;

    #[test]
    fn maps_fprint_verify_status_values() {
        assert_eq!(
            verify_status("verify-match", true),
            FingerprintStatus::Accepted
        );
        assert_eq!(
            verify_status("verify-no-match", true),
            FingerprintStatus::NotRecognized
        );
        assert_eq!(
            verify_status("verify-disconnected", false),
            FingerprintStatus::Unavailable
        );
        assert_eq!(verify_status("unknown", true), FingerprintStatus::Error);
    }
}
