//! Stress and concurrency tests — equivalent to edge-js stress/test.js

mod common;

use std::collections::HashMap;
use dotbridge::ClrValue;

// =============================================================================
// Many sequential calls
// =============================================================================

#[tokio::test]
async fn stress_100_sequential_calls() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input + 1; }"#)
        .unwrap();

    for i in 0..100 {
        let result = func.call(ClrValue::Int32(i)).await.unwrap();
        assert_eq!(result, ClrValue::Int32(i + 1), "failed at iteration {i}");
    }
}

#[test]
fn stress_100_sequential_sync_calls() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input * 2; }"#)
        .unwrap();

    for i in 0..100 {
        let result = func.call_sync(ClrValue::Int32(i)).unwrap();
        assert_eq!(result, ClrValue::Int32(i * 2), "failed at iteration {i}");
    }
}

// =============================================================================
// Concurrent async calls
// =============================================================================

#[tokio::test]
async fn stress_concurrent_calls() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input * 3; }"#)
        .unwrap();

    let mut handles = Vec::new();
    for i in 0..50 {
        let result_future = func.call(ClrValue::Int32(i));
        handles.push((i, result_future));
    }

    for (i, future) in handles {
        let result = future.await.unwrap();
        assert_eq!(result, ClrValue::Int32(i * 3), "failed for input {i}");
    }
}

// =============================================================================
// Large payload stress
// =============================================================================

#[tokio::test]
async fn stress_large_json_payload() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return input; }"#)
        .unwrap();

    // Build a large nested object (~8KB like edge-js stress test)
    let mut payload = HashMap::new();
    for i in 0..100 {
        let key = format!("key_{i}");
        let value = format!("value_{}_with_some_padding_to_make_it_larger", i);
        payload.insert(key, ClrValue::String(value));
    }

    let result = func
        .call(ClrValue::Object(payload.clone()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::Object(payload));
}

#[tokio::test]
async fn stress_large_buffer() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return input; }"#)
        .unwrap();

    // 1MB buffer
    let buf = vec![0xABu8; 1024 * 1024];
    let result = func.call(ClrValue::Buffer(buf.clone())).await.unwrap();
    assert_eq!(result, ClrValue::Buffer(buf));
}

// =============================================================================
// Many function compilations
// =============================================================================

#[test]
fn stress_compile_many_functions() {
    let runtime = common::init_runtime();

    for i in 0..20 {
        let source = format!(r#"async (input) => {{ return (int)input + {i}; }}"#);
        let func = runtime.func_from_source(&source).unwrap();
        let result = func.call_sync(ClrValue::Int32(1)).unwrap();
        assert_eq!(result, ClrValue::Int32(1 + i), "failed at compilation {i}");
    }
}

// =============================================================================
// Callback stress
// =============================================================================

#[test]
fn stress_register_many_callbacks() {
    use dotbridge::runtime::{register_callback, unregister_callback, invoke_callback};

    let mut ids = Vec::new();
    for i in 0..100 {
        let id = register_callback(move |_| Ok(ClrValue::Int32(i)));
        ids.push((i, id));
    }

    for (expected, id) in &ids {
        let result = invoke_callback(*id, ClrValue::Null).unwrap();
        assert_eq!(result, ClrValue::Int32(*expected));
    }

    for (_, id) in &ids {
        unregister_callback(*id);
    }

    // All should be gone
    for (_, id) in &ids {
        assert!(invoke_callback(*id, ClrValue::Null).is_err());
    }
}

// =============================================================================
// Rapid sequential exception handling
// =============================================================================

#[tokio::test]
async fn stress_exceptions_dont_leak() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                int x = (int)input;
                if (x % 2 == 0) {
                    throw new System.Exception("even error");
                }
                return x;
            }"#,
        )
        .unwrap();

    for i in 0..50 {
        let result = func.call(ClrValue::Int32(i)).await;
        if i % 2 == 0 {
            assert!(result.is_err(), "even {i} should error");
        } else {
            assert_eq!(result.unwrap(), ClrValue::Int32(i), "odd {i} should succeed");
        }
    }
}

// =============================================================================
// Mixed operation stress
// =============================================================================

#[tokio::test]
async fn stress_mixed_operations() {
    let runtime = common::init_runtime();

    let add = runtime
        .func_from_source(r#"async (input) => { return (int)input + 1; }"#)
        .unwrap();
    let echo = runtime
        .func_from_source(r#"async (input) => { return input; }"#)
        .unwrap();

    for i in 0..30 {
        // Alternate between different operations
        match i % 3 {
            0 => {
                let result = add.call(ClrValue::Int32(i)).await.unwrap();
                assert_eq!(result, ClrValue::Int32(i + 1));
            }
            1 => {
                let result = add.call_sync(ClrValue::Int32(i)).unwrap();
                assert_eq!(result, ClrValue::Int32(i + 1));
            }
            2 => {
                let mut map = HashMap::new();
                map.insert("i".to_string(), ClrValue::Int32(i));
                let result = echo.call(ClrValue::Object(map.clone())).await.unwrap();
                assert_eq!(result, ClrValue::Object(map));
            }
            _ => unreachable!(),
        }
    }
}
