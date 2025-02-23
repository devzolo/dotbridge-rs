use dotbridge::{ClrValue, DotBridgeRuntime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the .NET runtime
    let runtime = DotBridgeRuntime::new()?;

    // Register a Rust callback that .NET can call
    let callback_id = runtime.register_callback(|input| {
        println!("Rust callback invoked with: {:?}", input);
        Ok(ClrValue::String("Hello from Rust callback!".into()))
    });

    // Create a .NET function that will call our Rust callback
    let dotnet_func = runtime.func_from_source(r#"
        async (input) => {
            // In a real implementation, the callback ID would be used
            // to invoke the Rust function from .NET
            return "Callback registered successfully";
        }
    "#)?;

    let result = dotnet_func.call(ClrValue::Int32(callback_id as i32)).await?;
    println!("Result: {:?}", result);

    // Cleanup
    runtime.unregister_callback(callback_id);

    Ok(())
}
