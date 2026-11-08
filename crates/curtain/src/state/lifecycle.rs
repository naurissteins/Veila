use super::CurtainApp;
use anyhow::{Context, Result};
use wayland_protocols_wlr::output_power_management::v1::client::zwlr_output_power_v1;
impl CurtainApp {
    pub(crate) fn unlocked(&self) -> bool {
        self.unlock_authorized && self.session_lock.is_none() && !self.session_finished
    }

    pub(crate) fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    // only a current PAM verdict or the authenticated control channel may authorize unlock
    pub(crate) fn authorize_unlock(&mut self) {
        self.unlock_authorized = true;
    }

    pub(crate) fn request_exit_from_signal(&mut self) {
        self.exit_requested = true;
    }

    pub(crate) fn can_stop(&self) -> bool {
        self.failure_reason.is_some()
            || (self.exit_requested && (self.session_locked || self.session_finished))
    }

    pub(crate) fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    pub(crate) fn shutdown(&mut self) -> Result<()> {
        self.render_profiler.log_summary();

        if self.session_finished {
            self.session_lock.take();
            return Ok(());
        }

        if self.outputs_powered_off() || self.secondary_outputs_powered_off {
            let _ = self.set_outputs_power_mode(zwlr_output_power_v1::Mode::On);
            self.secondary_outputs_powered_off = false;
        }

        let Some(session_lock) = self.session_lock.take() else {
            return Ok(());
        };

        if !self.unlock_authorized {
            tracing::error!(
                failure_reason = self.failure_reason.as_deref(),
                "curtain is exiting without an authorized unlock; leaving the session locked"
            );
            // leaked on purpose: dropping would send destroy() and can raise invalid_destroy
            std::mem::forget(session_lock);
            return Ok(());
        }

        if session_lock.is_locked() {
            tracing::info!("releasing session lock");
            session_lock.unlock();
            self.connection
                .roundtrip()
                .context("failed to roundtrip after unlocking session")?;
        }

        Ok(())
    }
}
