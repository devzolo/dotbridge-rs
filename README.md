<p align="center">
  <img src="https://img.shields.io/crates/v/dotbridge?style=flat-square&color=blue" alt="crates.io" />
  <img src="https://img.shields.io/crates/l/dotbridge?style=flat-square" alt="license" />
  <img src="https://img.shields.io/badge/rust-1.70%2B-orange?style=flat-square" alt="rust version" />
  <img src="https://img.shields.io/badge/.NET-8.0%2B-blueviolet?style=flat-square" alt=".NET version" />
  <img src="https://img.shields.io/badge/platform-windows%20%7C%20linux%20%7C%20macos-lightgrey?style=flat-square" alt="platform" />
</p>

# dotbridge

**Run .NET code from Rust** — seamless, high-performance Rust/.NET interop with zero boilerplate.

```rust
use dotbridge::{ClrValue, DotBridgeRuntime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = DotBridgeRuntime::new()?;

    let greet = runtime.func_from_source(r#"
        async (input) => {
            return "Hello from .NET! You said: " + input;
        }
    "#)?;

    let result = greet.call(ClrValue::String("hi from Rust".into())).await?;
    println!("{:?}", result); // String("Hello from .NET! You said: hi from Rust")

    Ok(())
}
```

dotbridge lets you write inline C# (or load pre-compiled .NET assemblies) and call them from Rust as if they were native functions. No code generation, no COM, no HTTP — just direct in-process interop over a fast binary wire protocol.

---

## Features

- **Inline C# compilation** — Write C# lambdas or full classes directly in Rust source, compiled at runtime via Roslyn
- **Pre-compiled assembly loading** — Load any .NET DLL and call methods by type/method name
- **Binary wire protocol** — Fast serialization between Rust and .NET (no JSON overhead)
- **Full type marshaling** — Primitives, strings, buffers, arrays, objects, dates, decimals, GUIDs, and more
- **Bidirectional callbacks** — Register Rust functions that .NET code can invoke
- **Derive macro** — `#[derive(DotNetMarshal)]` for automatic struct marshaling
- **Async/sync** — Both `async fn call()` (tokio) and `fn call_sync()` APIs
- **Automatic resource cleanup** — .NET GCHandles are freed on `Drop`
- **Isolated assembly loading** — Custom `AssemblyLoadContext` per assembly with deps.json resolution
- **Structured exceptions** — .NET exceptions propagate to Rust with message, stack trace, and type name
- **Cross-platform** — Windows, Linux, and macOS

## Crates

| Crate | Description |
|---|---|
| [`dotbridge`](https://crates.io/crates/dotbridge) | High-level API — the one you want |
| [`dotbridge-sys`](https://crates.io/crates/dotbridge-sys) | Raw FFI bindings to CoreCLR hosting APIs (hostfxr) |
| [`dotbridge-derive`](https://crates.io/crates/dotbridge-derive) | Proc macros (`#[derive(DotNetMarshal)]`) |

## Requirements

- **Rust** 1.70+
- **.NET SDK** 8.0+ (or any installed .NET runtime 6.0+)
- **Supported platforms**: Windows x64, Linux x64, macOS x64/arm64

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dotbridge = "0.1"
tokio = { version = "1", features = ["rt", "macros"] }
```

The .NET bootstrap assembly is compiled automatically by `build.rs` during `cargo build`. Make sure `dotnet` is in your PATH.

## Quick Start

### Inline C# (Lambda)

```rust
use dotbridge::{ClrValue, DotBridgeRuntime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = DotBridgeRuntime::new()?;

    let add = runtime.func_from_source(r#"
        async (input) => {
            return (int)input + 1;
        }
    "#)?;

    let result = add.call(ClrValue::Int32(41)).await?;
    assert_eq!(result, ClrValue::Int32(42));

    // Sync variant (no tokio needed)
    let result = add.call_sync(ClrValue::Int32(99))?;
    assert_eq!(result, ClrValue::Int32(100));

    Ok(())
}
```

### Inline C# (Full Class)

```rust
let counter = runtime.func_from_source(r#"
    using System;
    using System.Threading.Tasks;

    public class Startup
    {
        private int count = 0;

        public async Task<object?> Invoke(object? input)
        {
            count++;
            return count;
        }
    }
"#)?;

let r1 = counter.call_sync(ClrValue::Null)?; // Int32(1)
let r2 = counter.call_sync(ClrValue::Null)?; // Int32(2)
```

### Pre-Compiled Assembly

```rust
let my_func = runtime.func_from_assembly(
    "path/to/MyLibrary.dll",
    "MyNamespace.Startup, MyLibrary",
    "Invoke",
)?;

let result = my_func.call(ClrValue::String("hello".into())).await?;
```

### Structured Data with Objects

```rust
use std::collections::HashMap;

let mut input = HashMap::new();
input.insert("name".to_string(), ClrValue::String("Alice".into()));
input.insert("age".to_string(), ClrValue::Int32(30));

let process = runtime.func_from_source(r#"
    async (dynamic input) => {
        return $"Hello {input.name}, you are {input.age} years old!";
    }
"#)?;

let result = process.call(ClrValue::Object(input)).await?;
```

### Derive Macro for Struct Marshaling

```rust
use dotbridge::DotNetMarshal;

#[derive(DotNetMarshal)]
struct Person {
    name: String,
    age: i32,
    scores: Vec<f64>,
    address: Option<String>,
}

let person = Person {
    name: "Alice".into(),
    age: 30,
    scores: vec![95.0, 87.5],
    address: Some("123 Main St".into()),
};

// Automatically converts to/from ClrValue::Object
let clr = person.to_clr_value();
let back = Person::from_clr_value(&clr)?;
```

### Rust Callbacks from .NET

```rust
let callback_id = runtime.register_callback(|input| {
    println!("Called from .NET with: {:?}", input);
    Ok(ClrValue::String("response from Rust".into()))
});

// Pass callback_id to .NET code that can invoke it
// ...

runtime.unregister_callback(callback_id);
```

### Compile with Extra Assembly References

```rust
let func = runtime.func_from_source_with_references(
    r#"
        async (input) => {
            var client = new Newtonsoft.Json.Linq.JObject();
            return client.ToString();
        }
    "#,
    &["path/to/Newtonsoft.Json.dll"],
)?;
```

## Type Mapping

| Rust (`ClrValue`) | .NET Type | Notes |
|---|---|---|
| `Null` | `null` | |
| `String(String)` | `string` | UTF-8 |
| `Boolean(bool)` | `bool` | |
| `Int32(i32)` | `int` | |
| `UInt32(u32)` | `uint` | |
| `Int64(i64)` | `long` | Serialized as `double` on wire |
| `Double(f64)` | `double` | |
| `Float(f32)` | `float` | Serialized as `double` on wire |
| `DateTime(i64)` | `DateTime` | Milliseconds since Unix epoch |
| `Guid(String)` | `Guid` | String representation |
| `Decimal(String)` | `decimal` | String for lossless transport |
| `Buffer(Vec<u8>)` | `byte[]` | |
| `Array(Vec<ClrValue>)` | `object[]` | |
| `Object(HashMap)` | `ExpandoObject` | Dynamic access in C# |
| `Callback(handle)` | Rust callback | Bidirectional invocation |

## Configuration

```rust
use dotbridge::{DotBridgeConfig, DotBridgeRuntime};

let runtime = DotBridgeRuntime::with_config(DotBridgeConfig {
    target_framework: Some("net9.0".into()),
    bootstrap_dir: Some("/path/to/bootstrap".into()),
    assembly_search_paths: vec!["/extra/libs".into()],
    debug: true, // prints diagnostic info to stderr
    ..Default::default()
})?;
```

### Environment Variables

| Variable | Description |
|---|---|
| `DOTBRIDGE_DEBUG` | Enable debug output (set to any value) |
| `DOTBRIDGE_BOOTSTRAP_DIR` | Override bootstrap assembly directory |

## Error Handling

All .NET exceptions propagate as structured errors:

```rust
match func.call_sync(ClrValue::Null) {
    Ok(result) => println!("Success: {:?}", result),
    Err(DotBridgeError::DotNetException { message, stack_trace }) => {
        eprintln!("Exception: {message}");
        if let Some(trace) = stack_trace {
            eprintln!("Stack trace:\n{trace}");
        }
    }
    Err(e) => eprintln!("Other error: {e}"),
}
```

## Architecture

```
┌─────────────────────────────────────────────┐
│                  Rust App                    │
│                                             │
│  DotBridgeRuntime ──► DotBridgeFunc.call()  │
│         │                    │              │
│         ▼                    ▼              │
│    CSharpCompiler     Binary Wire Protocol  │
│         │              (serialize/deser)     │
│         ▼                    │              │
├─────────┼────────────────────┼──────────────┤
│    hostfxr FFI          FFI boundary        │
├─────────┼────────────────────┼──────────────┤
│         ▼                    ▼              │
│  Roslyn Compiler      WireProtocol.cs       │
│         │              (serialize/deser)     │
│         ▼                    │              │
│  AssemblyLoadContext ──► ManagedInvoker      │
│                                             │
│              .NET CoreCLR Runtime           │
└─────────────────────────────────────────────┘
```

**Key internals:**
- **hostfxr** — Loads and initializes the .NET runtime in-process
- **Binary wire protocol** — Custom format matching `ClrValue` ↔ C# types (no JSON, no protobuf)
- **[UnmanagedCallersOnly]** — C# entry points callable directly from native code
- **GCHandle** — Prevents .NET GC from collecting objects passed across the FFI boundary
- **Expression trees** — Compiled property accessors with `ConcurrentDictionary` cache for fast reflection
- **Custom AssemblyLoadContext** — Isolated loading with deps.json + NuGet + native library resolution

## Building from Source

```bash
# Clone the repository
git clone https://github.com/devzolo/dotbridge-rs.git
cd dotbridge-rs

# Build everything (Rust + C# bootstrap)
cargo build

# Run all tests (176 tests)
cargo test

# Run the hello example
cargo run --example hello
```

## Running Tests

```bash
# All tests
cargo test

# Specific test suites
cargo test --test test_csx          # Inline C# compilation
cargo test --test test_marshaling   # Wire protocol roundtrips
cargo test --test test_callbacks    # Rust↔.NET callbacks
cargo test --test test_errors       # Exception propagation
cargo test --test test_derive       # Derive macro
cargo test --test test_rust2net     # End-to-end Rust→.NET calls
cargo test --test test_stress       # Stress/concurrency tests
cargo test --test test_patterns     # Real-world usage patterns
```

## Contributing

Contributions are welcome! Feel free to open issues and pull requests.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Make sure all tests pass (`cargo test`)
4. Commit your changes
5. Push to the branch and open a Pull Request

## License

This project is licensed under the [MIT License](LICENSE).

## Author

**devzolo** — [github.com/devzolo](https://github.com/devzolo)

---

<p align="center">
  <sub>Built with Rust and .NET</sub>
</p>
