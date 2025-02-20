use thiserror::Error;

#[derive(Debug, Error)]
pub enum SysError {
    #[error("failed to load native library: {0}")]
    LibraryLoad(#[from] libloading::Error),

    #[error("hostfxr not found. Is .NET installed? Searched: {searched_paths:?}")]
    HostFxrNotFound { searched_paths: Vec<String> },

    #[error("hostfxr function '{name}' not found")]
    SymbolNotFound { name: String },

    #[error("hostfxr call failed with status code: 0x{0:08X}")]
    HostFxrError(i32),

    #[error("failed to initialize CoreCLR runtime")]
    RuntimeInitFailed,

    #[error("delegate type {0} not available")]
    DelegateNotAvailable(String),

    #[error("invalid UTF-8 in path: {0}")]
    InvalidPath(String),

    #[error("{0}")]
    Other(String),
}
