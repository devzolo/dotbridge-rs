//! Tests for Rust-to-.NET calls — equivalent to edge-js 102_node2net.js and 105_node2net_sync.js

mod common;

use dotbridge::ClrValue;

// =============================================================================
// Basic async calls (102_node2net.js equivalents)
// =============================================================================

#[tokio::test]
async fn call_hello_world() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return "Hello, World!"; }"#)
        .unwrap();

    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::String("Hello, World!".into()));
}

#[tokio::test]
async fn call_with_string_input() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return "Hello, " + input?.ToString() + "!"; }"#)
        .unwrap();

    let result = func.call(ClrValue::String("Rust".into())).await.unwrap();
    assert_eq!(result, ClrValue::String("Hello, Rust!".into()));
}

#[tokio::test]
async fn call_with_int_input() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input + 1; }"#)
        .unwrap();

    let result = func.call(ClrValue::Int32(41)).await.unwrap();
    assert_eq!(result, ClrValue::Int32(42));
}

#[tokio::test]
async fn call_with_double_input() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (double)input * 2.0; }"#)
        .unwrap();

    let result = func.call(ClrValue::Double(3.14)).await.unwrap();
    if let ClrValue::Double(n) = result {
        assert!((n - 6.28).abs() < 0.001);
    } else {
        panic!("expected Double, got {result:?}");
    }
}

#[tokio::test]
async fn call_with_bool_input() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return !(bool)input; }"#)
        .unwrap();

    let result = func.call(ClrValue::Boolean(true)).await.unwrap();
    assert_eq!(result, ClrValue::Boolean(false));
}

#[tokio::test]
async fn call_with_null_input() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return input == null ? "was null" : "was not null"; }"#)
        .unwrap();

    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::String("was null".into()));
}

#[tokio::test]
async fn call_returning_null() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return null; }"#)
        .unwrap();

    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::Null);
}

#[tokio::test]
async fn call_with_empty_string() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (string)input; }"#)
        .unwrap();

    let result = func.call(ClrValue::String(String::new())).await.unwrap();
    assert_eq!(result, ClrValue::String(String::new()));
}

#[tokio::test]
async fn call_with_unicode_string() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (string)input; }"#)
        .unwrap();

    let input = "こんにちは世界 🌍 ñ ü ö";
    let result = func
        .call(ClrValue::String(input.into()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::String(input.into()));
}

#[tokio::test]
async fn call_with_buffer_input() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var bytes = (byte[])input;
                return bytes.Length;
            }"#,
        )
        .unwrap();

    let result = func
        .call(ClrValue::Buffer(vec![1, 2, 3, 4, 5]))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::Int32(5));
}

#[tokio::test]
async fn call_with_empty_buffer() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var bytes = (byte[])input;
                return bytes.Length;
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::Buffer(vec![])).await.unwrap();
    assert_eq!(result, ClrValue::Int32(0));
}

// =============================================================================
// Synchronous calls (105_node2net_sync.js equivalents)
// =============================================================================

#[test]
fn call_sync_hello_world() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return "Hello sync!"; }"#)
        .unwrap();

    let result = func.call_sync(ClrValue::Null).unwrap();
    assert_eq!(result, ClrValue::String("Hello sync!".into()));
}

#[test]
fn call_sync_with_int() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input + 1; }"#)
        .unwrap();

    let result = func.call_sync(ClrValue::Int32(99)).unwrap();
    assert_eq!(result, ClrValue::Int32(100));
}

#[test]
fn call_sync_and_async_same_func() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input * 2; }"#)
        .unwrap();

    // Sync call
    let sync_result = func.call_sync(ClrValue::Int32(5)).unwrap();
    assert_eq!(sync_result, ClrValue::Int32(10));

    // Async call on the same function
    let rt = tokio::runtime::Runtime::new().unwrap();
    let async_result = rt.block_on(func.call(ClrValue::Int32(7))).unwrap();
    assert_eq!(async_result, ClrValue::Int32(14));
}

#[test]
fn call_sync_multiple_times() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input + 1; }"#)
        .unwrap();

    for i in 0..10 {
        let result = func.call_sync(ClrValue::Int32(i)).unwrap();
        assert_eq!(result, ClrValue::Int32(i + 1));
    }
}
