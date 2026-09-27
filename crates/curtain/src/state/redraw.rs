use veila_renderer::shm::FrameResult;
use veila_ui::WidgetKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RedrawKind {
    AuthDirty,
    Widget(WidgetKind),
    Full,
}

#[derive(Debug, Default)]
pub(crate) struct PendingRedraw(Option<RedrawKind>);

impl PendingRedraw {
    pub(crate) fn request(&mut self, requested: RedrawKind) {
        self.0 = Some(match (self.0, requested) {
            (None, requested) => requested,
            (Some(RedrawKind::Full), _) | (_, RedrawKind::Full) => RedrawKind::Full,
            (Some(RedrawKind::Widget(old)), RedrawKind::Widget(new)) if old == new => {
                RedrawKind::Widget(old)
            }
            (Some(RedrawKind::AuthDirty), RedrawKind::AuthDirty) => RedrawKind::AuthDirty,
            _ => RedrawKind::Full,
        });
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

    #[test]
    fn different_widget_redraws_promote_to_full_frame() {
        let mut pending = PendingRedraw::default();
        pending.request(RedrawKind::Widget(veila_ui::WidgetKind::Header));
        pending.request(RedrawKind::Widget(veila_ui::WidgetKind::Media));

        assert_eq!(pending.take(), Some(RedrawKind::Full));
    }

    #[test]
    fn auth_and_widget_redraws_promote_to_full_frame() {
        let mut pending = PendingRedraw::default();
        pending.request(RedrawKind::AuthDirty);
        pending.request(RedrawKind::Widget(veila_ui::WidgetKind::Indicators));

        assert_eq!(pending.take(), Some(RedrawKind::Full));
    }
}
