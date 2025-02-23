//! Tests for call patterns and advanced scenarios — equivalent to edge-js 201_patterns.js

mod common;

use dotbridge::ClrValue;

// =============================================================================
// Lambda with closure / state (edge-js pattern: counter increment)
// =============================================================================

#[tokio::test]
async fn lambda_with_closure_state() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"
            using System.Threading.Tasks;
            public class Startup {
                private int counter = 0;
                public async Task<object> Invoke(object input) {
                    counter++;
                    return counter;
                }
            }
            "#,
        )
        .unwrap();

    let r1 = func.call(ClrValue::Null).await.unwrap();
    let r2 = func.call(ClrValue::Null).await.unwrap();
    let r3 = func.call(ClrValue::Null).await.unwrap();

    assert_eq!(r1, ClrValue::Int32(1));
    assert_eq!(r2, ClrValue::Int32(2));
    assert_eq!(r3, ClrValue::Int32(3));
}

// =============================================================================
// Sync + async interleaved on the same function
// =============================================================================

#[tokio::test]
async fn sync_and_async_interleaved() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input + 10; }"#)
        .unwrap();

    let sync1 = func.call_sync(ClrValue::Int32(1)).unwrap();
    let async1 = func.call(ClrValue::Int32(2)).await.unwrap();
    let sync2 = func.call_sync(ClrValue::Int32(3)).unwrap();
    let async2 = func.call(ClrValue::Int32(4)).await.unwrap();

    assert_eq!(sync1, ClrValue::Int32(11));
    assert_eq!(async1, ClrValue::Int32(12));
    assert_eq!(sync2, ClrValue::Int32(13));
    assert_eq!(async2, ClrValue::Int32(14));
}

// =============================================================================
// Multiple functions created and used together
// =============================================================================

#[tokio::test]
async fn multiple_functions_independent() {
    let runtime = common::init_runtime();

    let square = runtime
        .func_from_source(r#"async (input) => { int x = (int)input; return x * x; }"#)
        .unwrap();
    let double = runtime
        .func_from_source(r#"async (input) => { return (int)input * 2; }"#)
        .unwrap();
    let negate = runtime
        .func_from_source(r#"async (input) => { return -(int)input; }"#)
        .unwrap();

    assert_eq!(square.call(ClrValue::Int32(5)).await.unwrap(), ClrValue::Int32(25));
    assert_eq!(double.call(ClrValue::Int32(5)).await.unwrap(), ClrValue::Int32(10));
    assert_eq!(negate.call(ClrValue::Int32(5)).await.unwrap(), ClrValue::Int32(-5));
}

// =============================================================================
// .NET Task / async patterns
// =============================================================================

#[tokio::test]
async fn dotnet_task_delay() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                await System.Threading.Tasks.Task.Delay(10);
                return "delayed";
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::String("delayed".into()));
}

#[tokio::test]
async fn dotnet_task_with_computation() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                int n = (int)input;
                int sum = 0;
                for (int i = 1; i <= n; i++) {
                    sum += i;
                }
                await System.Threading.Tasks.Task.Yield();
                return sum;
            }"#,
        )
        .unwrap();

    // Sum of 1..100 = 5050
    let result = func.call(ClrValue::Int32(100)).await.unwrap();
    assert_eq!(result, ClrValue::Int32(5050));
}

// =============================================================================
// LINQ and System.Collections usage
// =============================================================================

#[tokio::test]
async fn dotnet_linq_operations() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var list = new System.Collections.Generic.List<int> { 1, 2, 3, 4, 5 };
                int sum = 0;
                foreach (var x in list) sum += x;
                return sum;
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::Int32(15));
}

// =============================================================================
// String processing in .NET
// =============================================================================

#[tokio::test]
async fn dotnet_string_processing() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                string s = (string)input;
                return s.ToUpper().Trim().Replace("HELLO", "HI");
            }"#,
        )
        .unwrap();

    let result = func
        .call(ClrValue::String("  hello world  ".into()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::String("HI WORLD".into()));
}

// =============================================================================
// .NET returning nested structures
// =============================================================================

#[tokio::test]
async fn dotnet_returns_nested_structure() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var result = new System.Collections.Generic.Dictionary<string, object>();
                result["name"] = "test";
                result["items"] = new object[] { 1, 2, 3 };
                var nested = new System.Collections.Generic.Dictionary<string, object>();
                nested["key"] = "value";
                result["nested"] = nested;
                return result;
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::Null).await.unwrap();
    if let ClrValue::Object(map) = result {
        assert_eq!(map.get("name"), Some(&ClrValue::String("test".into())));
        if let Some(ClrValue::Array(items)) = map.get("items") {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0], ClrValue::Int32(1));
        } else {
            panic!("expected Array for 'items'");
        }
        if let Some(ClrValue::Object(nested)) = map.get("nested") {
            assert_eq!(nested.get("key"), Some(&ClrValue::String("value".into())));
        } else {
            panic!("expected Object for 'nested'");
        }
    } else {
        panic!("expected Object, got {result:?}");
    }
}
