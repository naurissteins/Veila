use thiserror::Error;

/// Shared error type for config and IPC bootstrap code.
#[derive(Debug, Error)]
pub enum KwylockError {
    #[error("failed to parse config: {0}")]
    Config(#[from] toml::de::Error),
    #[error("failed to encode or decode ipc message: {0}")]
    IpcCodec(#[from] serde_json::Error),
}

/// Common result type for shared Kwylock libraries.
pub type Result<T> = std::result::Result<T, KwylockError>;
