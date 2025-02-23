use dotbridge::{ClrValue, DotBridgeConfig, DotBridgeRuntime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the .NET runtime (build.rs auto-compiles the bootstrap)
    let runtime = DotBridgeRuntime::with_config(DotBridgeConfig {
        debug: true,
        ..Default::default()
    })?;

    // Create a .NET function from inline C# code
    let hello = runtime.func_from_source(r#"
        async (input) => {
            return "Hello from .NET! Input was: " + input?.ToString();
        }
    "#)?;

    // Call it asynchronously
    let result = hello.call(ClrValue::String("Rust".into())).await?;
    println!("Result: {:?}", result);

    // Call with different types
    let add = runtime.func_from_source(r#"
        async (input) => {
            return (int)input + 1;
        }
    "#)?;

    let result = add.call(ClrValue::Int32(41)).await?;
    println!("41 + 1 = {:?}", result);

    // Sync call
    let result = add.call_sync(ClrValue::Int32(99))?;
    println!("99 + 1 = {:?}", result);

    Ok(())
}
