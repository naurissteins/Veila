use thiserror::Error;

/// Shared error type for config and IPC bootstrap code.
#[derive(Debug, Error)]
pub enum VeilaError {
    #[error("failed to parse config: {0}")]
    Config(#[from] toml::de::Error),
    #[error("failed to load config: {0}")]
    ConfigIo(#[from] std::io::Error),
    #[error("unknown theme preset '{0}'")]
    ThemeNotFound(String),
    #[error("invalid theme preset name '{0}'")]
    InvalidThemeName(String),
    #[error("failed to encode or decode ipc message: {0}")]
    IpcCodec(#[from] serde_json::Error),
}

/// Common result type for shared Veila libraries.
pub type Result<T> = std::result::Result<T, VeilaError>;
