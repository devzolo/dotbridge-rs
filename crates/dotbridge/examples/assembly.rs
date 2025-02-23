use dotbridge::{ClrValue, DotBridgeRuntime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the .NET runtime
    let runtime = DotBridgeRuntime::new()?;

    // Load a pre-compiled .NET assembly
    let my_func = runtime.func_from_assembly(
        "path/to/MyAssembly.dll",
        "MyNamespace.Startup, MyAssembly",
        "Invoke",
    )?;

    // Call it with structured data
    let mut input = std::collections::HashMap::new();
    input.insert("name".to_string(), ClrValue::String("dotbridge".into()));
    input.insert("count".to_string(), ClrValue::Int32(42));

    let result = my_func.call(ClrValue::Object(input)).await?;
    println!("Result: {:?}", result);

    Ok(())
}
