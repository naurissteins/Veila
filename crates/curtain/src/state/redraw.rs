use veila_renderer::shm::FrameResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RedrawKind {
    AuthDirty,
    Full,
}

#[derive(Debug, Default)]
pub(crate) struct PendingRedraw(Option<RedrawKind>);

impl PendingRedraw {
    pub(crate) fn request(&mut self, requested: RedrawKind) {
        if matches!(requested, RedrawKind::Full) || self.0.is_none() {
            self.0 = Some(requested);
        }
    }

    pub(crate) fn satisfy(&mut self, rendered: RedrawKind) {
        if matches!(rendered, RedrawKind::Full) || self.0 == Some(rendered) {
            self.0 = None;
        }
    }

    pub(crate) fn take(&mut self) -> Option<RedrawKind> {
        self.0.take()
    }

    pub(crate) fn requires_full(&self) -> bool {
        self.0 == Some(RedrawKind::Full)
    }

    pub(crate) fn record_result(&mut self, rendered: RedrawKind, result: FrameResult) -> bool {
        match result {
            FrameResult::Committed => {
                self.satisfy(rendered);
                true
            }
            FrameResult::Skipped => {
                self.request(rendered);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use veila_renderer::shm::FrameResult;

    use super::{PendingRedraw, RedrawKind};

    #[test]
    fn full_redraw_supersedes_a_pending_auth_redraw() {
        let mut pending = PendingRedraw::default();
        pending.request(RedrawKind::AuthDirty);
        pending.request(RedrawKind::Full);

        assert_eq!(pending.take(), Some(RedrawKind::Full));
    }

    #[test]
    fn auth_redraw_does_not_downgrade_a_pending_full_redraw() {
        let mut pending = PendingRedraw::default();
        pending.request(RedrawKind::Full);
        pending.request(RedrawKind::AuthDirty);

        assert_eq!(pending.take(), Some(RedrawKind::Full));
    }

    #[test]
    fn auth_commit_does_not_satisfy_a_pending_full_redraw() {
        let mut pending = PendingRedraw::default();
        pending.request(RedrawKind::Full);
        pending.satisfy(RedrawKind::AuthDirty);

        assert_eq!(pending.take(), Some(RedrawKind::Full));
    }

    #[test]
    fn skipped_frame_stays_pending_until_a_commit() {
        let mut pending = PendingRedraw::default();

        assert!(!pending.record_result(RedrawKind::AuthDirty, FrameResult::Skipped));
        assert!(pending.record_result(RedrawKind::AuthDirty, FrameResult::Committed));
        assert_eq!(pending.take(), None);
    }
}
