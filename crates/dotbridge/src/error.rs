use thiserror::Error;

#[derive(Debug, Error)]
pub enum DotBridgeError {
    #[error("runtime error: {0}")]
    Runtime(#[from] dotbridge_sys::SysError),

    #[error("marshal error: {0}")]
    MarshalError(String),

    #[error(".NET exception: {message}")]
    DotNetException { message: String, stack_trace: Option<String> },

    #[error("compilation failed: {0}")]
    CompilationError(String),

    #[error("runtime not initialized")]
    NotInitialized,

    #[error("async task was cancelled")]
    TaskCancelled,

    #[error("async task faulted: {0}")]
    TaskFaulted(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    #[error("callback error: {0}")]
    CallbackError(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
