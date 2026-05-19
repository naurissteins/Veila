use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedSender;
use veila_common::FingerprintStatus;
use zbus::{proxy, zvariant::OwnedObjectPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyOutcome {
    Matched,
    NotMatched,
    Unavailable,
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
) -> Result<VerifyOutcome> {
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

    let result = verify_claimed_device(username, &device, status_sender).await;
    if let Err(error) = device.release().await {
        tracing::debug!("failed to release fprintd device: {error}");
    }
    result
}

async fn verify_claimed_device(
    username: &str,
    device: &FprintDeviceProxy<'_>,
    status_sender: &UnboundedSender<Option<FingerprintStatus>>,
) -> Result<VerifyOutcome> {
    let enrolled = device
        .list_enrolled_fingers(username)
        .await
        .context("failed to list enrolled fingerprints")?;
    if enrolled.is_empty() {
        let _ = status_sender.send(Some(FingerprintStatus::NoEnrolledFingers));
        return Ok(VerifyOutcome::Unavailable);
    }

    let mut stream = device
        .receive_verify_status()
        .await
        .context("failed to subscribe to fprintd verification status")?;
    let _ = status_sender.send(Some(FingerprintStatus::Ready));
    device
        .verify_start("any")
        .await
        .context("failed to start fprintd verification")?;

    while let Some(signal) = stream.next().await {
        let args = signal
            .args()
            .context("failed to decode fprintd verification status")?;
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
        if let Err(error) = device.verify_stop().await {
            tracing::debug!("failed to stop fprintd verification: {error}");
        }
        return Ok(outcome);
    }

    Ok(VerifyOutcome::Unavailable)
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
