use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Default)]
pub(crate) struct Authority(Arc<AtomicU64>);

impl Authority {
    pub(crate) fn issue(&self) -> Authorization {
        Authorization {
            epoch: self.0.load(Ordering::Acquire),
            current: Arc::clone(&self.0),
        }
    }
    pub(crate) fn revoke(&self) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

pub struct Authorization {
    epoch: u64,
    current: Arc<AtomicU64>,
}
impl Authorization {
    pub fn is_current(&self) -> bool {
        self.current.load(Ordering::Acquire) == self.epoch
    }
}
