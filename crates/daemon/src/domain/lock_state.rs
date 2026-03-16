#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    Unlocked,
    Locking,
    Locked,
    Unlocking,
}

impl LockState {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Locking | Self::Locked | Self::Unlocking)
    }
}

impl std::fmt::Display for LockState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unlocked => formatter.write_str("unlocked"),
            Self::Locking => formatter.write_str("locking"),
            Self::Locked => formatter.write_str("locked"),
            Self::Unlocking => formatter.write_str("unlocking"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LockState;

    #[test]
    fn reports_active_states() {
        assert!(!LockState::Unlocked.is_active());
        assert!(LockState::Locking.is_active());
        assert!(LockState::Locked.is_active());
        assert!(LockState::Unlocking.is_active());
    }
}
