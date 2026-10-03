use std::{path::PathBuf, time::Instant};

use tokio::{
    net::UnixListener,
    time::{Instant as TokioInstant, sleep_until},
};
use veila_common::{
    elapsed_ms, elapsed_us,
    ipc::{CurtainStartupMessage, LockLatencyReport},
};

use super::startup::{log_latency_report, read_startup_message};

pub(crate) struct RichReadiness {
    pub(super) listener: UnixListener,
    pub(super) path: PathBuf,
    pub(super) deadline: TokioInstant,
    pub(super) trigger: &'static str,
    pub(super) activation_started_at: Instant,
    pub(super) ready_wait_started_at: Instant,
    pub(super) report: Option<LockLatencyReport>,
}

impl RichReadiness {
    pub(crate) async fn wait(mut self) -> Option<LockLatencyReport> {
        let read = async {
            loop {
                let (stream, _) = match self.listener.accept().await {
                    Ok(accepted) => accepted,
                    Err(error) => {
                        tracing::warn!(
                            "startup notification failed: {error}; preserving the locked curtain"
                        );
                        return None;
                    }
                };
                match read_startup_message(stream).await {
                    Ok(Some(CurtainStartupMessage::Ready { latency_report })) => {
                        return Some(latency_report.map(|report| *report));
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!("failed to read curtain startup message: {error:#}")
                    }
                }
            }
        };
        let ready = tokio::select! {
            ready = read => ready,
            () = sleep_until(self.deadline) => {
                tracing::warn!("curtain readiness timed out after compositor lock confirmation; preserving the curtain");
                None
            }
        };
        let ready_wait_elapsed_ms = elapsed_ms(self.ready_wait_started_at);
        let activation_elapsed_ms = elapsed_ms(self.activation_started_at);
        if let Some(report) = &mut self.report {
            report.curtain_ready_wait_ms = ready_wait_elapsed_ms;
            report.curtain_ready_wait_us = elapsed_us(self.ready_wait_started_at);
            report.activation_total_ms = activation_elapsed_ms;
            report.activation_total_us = elapsed_us(self.activation_started_at);
            report.curtain = ready.as_ref().and_then(Clone::clone);
            log_latency_report(report);
        }
        tracing::info!(
            trigger = self.trigger,
            ready = ready.is_some(),
            ready_wait_elapsed_ms,
            activation_elapsed_ms,
            "curtain startup completed; session considered locked"
        );
        self.report.take()
    }
}

impl Drop for RichReadiness {
    fn drop(&mut self) {
        // Readiness owns only its notification socket, never lock authority.
        let _ = std::fs::remove_file(&self.path);
    }
}
