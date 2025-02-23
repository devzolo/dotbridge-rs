use dotbridge::{DotBridgeConfig, DotBridgeRuntime};

/// Initialize a DotBridgeRuntime for integration tests.
/// Panics if .NET runtime cannot be loaded (e.g., missing dotnet SDK).
pub fn init_runtime() -> DotBridgeRuntime {
    DotBridgeRuntime::with_config(DotBridgeConfig {
        debug: std::env::var("DOTBRIDGE_DEBUG").is_ok(),
        ..Default::default()
    })
    .expect("Failed to initialize DotBridge runtime. Is .NET SDK installed?")
}
