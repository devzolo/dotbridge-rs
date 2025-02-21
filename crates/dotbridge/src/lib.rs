pub mod error;
pub mod marshal;
pub mod runtime;
pub mod func;
pub mod compiler;

pub use dotbridge_derive::DotNetMarshal;
pub use error::DotBridgeError;
pub use func::DotBridgeFunc;
pub use marshal::{ClrValue, FromClrValue, ToClrValue};
pub use runtime::{DotBridgeConfig, DotBridgeRuntime};

/// Create a .NET function from inline C# source code.
///
/// # Example
/// ```no_run
/// # use dotbridge::{ClrValue, DotBridgeRuntime, DotBridgeError};
/// # async fn example() -> Result<(), DotBridgeError> {
/// let runtime = DotBridgeRuntime::new()?;
///
/// let add = dotbridge::func(&runtime, r#"
///     async (input) => {
///         return (int)input + 1;
///     }
/// "#)?;
///
/// let result = add.call(ClrValue::Int32(5)).await?;
/// assert_eq!(result, ClrValue::Int32(6));
/// # Ok(())
/// # }
/// ```
pub fn func(runtime: &DotBridgeRuntime, source: &str) -> Result<DotBridgeFunc, DotBridgeError> {
    runtime.func_from_source(source)
}

/// Create a .NET function from a pre-compiled assembly.
///
/// # Example
/// ```no_run
/// # use dotbridge::{ClrValue, DotBridgeRuntime, DotBridgeError};
/// # async fn example() -> Result<(), DotBridgeError> {
/// let runtime = DotBridgeRuntime::new()?;
///
/// let my_func = dotbridge::func_from_assembly(
///     &runtime,
///     "path/to/MyAssembly.dll",
///     "MyNamespace.Startup",
///     "Invoke",
/// )?;
///
/// let result = my_func.call(ClrValue::Null).await?;
/// # Ok(())
/// # }
/// ```
pub fn func_from_assembly(
    runtime: &DotBridgeRuntime,
    assembly_path: &str,
    type_name: &str,
    method_name: &str,
) -> Result<DotBridgeFunc, DotBridgeError> {
    runtime.func_from_assembly(assembly_path, type_name, method_name)
}
