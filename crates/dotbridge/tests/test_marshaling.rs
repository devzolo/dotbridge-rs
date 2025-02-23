//! Tests for data marshaling roundtrip through .NET — equivalent to edge-js 102/103 marshaling tests

mod common;

use std::collections::HashMap;
use dotbridge::ClrValue;

/// Helper: creates a .NET function that returns its input unchanged.
fn make_passthrough(runtime: &dotbridge::DotBridgeRuntime) -> dotbridge::DotBridgeFunc {
    runtime
        .func_from_source(r#"async (input) => { return input; }"#)
        .expect("passthrough function should compile")
}

// =============================================================================
// Primitive type roundtrips through .NET
// =============================================================================

#[tokio::test]
async fn marshal_int32_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Int32(42)).await.unwrap();
    assert_eq!(result, ClrValue::Int32(42));
}

#[tokio::test]
async fn marshal_int32_negative_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Int32(-999)).await.unwrap();
    assert_eq!(result, ClrValue::Int32(-999));
}

#[tokio::test]
async fn marshal_int32_zero_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Int32(0)).await.unwrap();
    assert_eq!(result, ClrValue::Int32(0));
}

#[tokio::test]
async fn marshal_bool_true_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Boolean(true)).await.unwrap();
    assert_eq!(result, ClrValue::Boolean(true));
}

#[tokio::test]
async fn marshal_bool_false_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Boolean(false)).await.unwrap();
    assert_eq!(result, ClrValue::Boolean(false));
}

#[tokio::test]
async fn marshal_double_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Double(3.14159)).await.unwrap();
    if let ClrValue::Double(n) = result {
        assert!((n - 3.14159).abs() < 1e-10);
    } else {
        panic!("expected Double, got {result:?}");
    }
}

#[tokio::test]
async fn marshal_string_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func
        .call(ClrValue::String("hello world".into()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::String("hello world".into()));
}

#[tokio::test]
async fn marshal_empty_string_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func
        .call(ClrValue::String(String::new()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::String(String::new()));
}

#[tokio::test]
async fn marshal_unicode_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let input = "こんにちは 🎉 Ñoño café résumé";
    let result = func
        .call(ClrValue::String(input.into()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::String(input.into()));
}

#[tokio::test]
async fn marshal_null_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::Null);
}

#[tokio::test]
async fn marshal_buffer_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let data = vec![0u8, 1, 2, 127, 128, 255];
    let result = func.call(ClrValue::Buffer(data.clone())).await.unwrap();
    assert_eq!(result, ClrValue::Buffer(data));
}

#[tokio::test]
async fn marshal_empty_buffer_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Buffer(vec![])).await.unwrap();
    assert_eq!(result, ClrValue::Buffer(vec![]));
}

// =============================================================================
// Complex type roundtrips through .NET
// =============================================================================

#[tokio::test]
async fn marshal_array_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let arr = ClrValue::Array(vec![
        ClrValue::Int32(1),
        ClrValue::String("two".into()),
        ClrValue::Boolean(true),
        ClrValue::Null,
    ]);
    let result = func.call(arr.clone()).await.unwrap();
    assert_eq!(result, arr);
}

#[tokio::test]
async fn marshal_empty_array_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Array(vec![])).await.unwrap();
    assert_eq!(result, ClrValue::Array(vec![]));
}

#[tokio::test]
async fn marshal_object_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);

    let mut map = HashMap::new();
    map.insert("name".to_string(), ClrValue::String("test".into()));
    map.insert("value".to_string(), ClrValue::Int32(42));
    map.insert("active".to_string(), ClrValue::Boolean(true));

    let result = func.call(ClrValue::Object(map.clone())).await.unwrap();
    assert_eq!(result, ClrValue::Object(map));
}

#[tokio::test]
async fn marshal_empty_object_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);
    let result = func.call(ClrValue::Object(HashMap::new())).await.unwrap();
    assert_eq!(result, ClrValue::Object(HashMap::new()));
}

#[tokio::test]
async fn marshal_nested_object_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);

    let mut inner = HashMap::new();
    inner.insert("x".to_string(), ClrValue::Int32(10));
    inner.insert("y".to_string(), ClrValue::Int32(20));

    let mut outer = HashMap::new();
    outer.insert("point".to_string(), ClrValue::Object(inner));
    outer.insert("label".to_string(), ClrValue::String("origin".into()));
    outer.insert(
        "tags".to_string(),
        ClrValue::Array(vec![
            ClrValue::String("a".into()),
            ClrValue::String("b".into()),
        ]),
    );

    let result = func
        .call(ClrValue::Object(outer.clone()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::Object(outer));
}

#[tokio::test]
async fn marshal_complex_payload_roundtrip() {
    let runtime = common::init_runtime();
    let func = make_passthrough(&runtime);

    let mut map = HashMap::new();
    map.insert("string".to_string(), ClrValue::String("hello".into()));
    map.insert("int".to_string(), ClrValue::Int32(42));
    map.insert("double".to_string(), ClrValue::Double(3.14));
    map.insert("bool".to_string(), ClrValue::Boolean(true));
    map.insert("null".to_string(), ClrValue::Null);
    map.insert("buffer".to_string(), ClrValue::Buffer(vec![1, 2, 3]));
    map.insert(
        "array".to_string(),
        ClrValue::Array(vec![
            ClrValue::Int32(1),
            ClrValue::Int32(2),
            ClrValue::Int32(3),
        ]),
    );

    let mut nested = HashMap::new();
    nested.insert("inner".to_string(), ClrValue::String("nested_value".into()));
    map.insert("nested".to_string(), ClrValue::Object(nested));

    let result = func
        .call(ClrValue::Object(map.clone()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::Object(map));
}

// =============================================================================
// .NET data creation tests (verify .NET can produce typed values)
// =============================================================================

#[tokio::test]
async fn net_returns_int() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return 42; }"#)
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::Int32(42));
}

#[tokio::test]
async fn net_returns_string() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return "from .NET"; }"#)
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::String("from .NET".into()));
}

#[tokio::test]
async fn net_returns_bool() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return true; }"#)
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::Boolean(true));
}

#[tokio::test]
async fn net_returns_double() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return 3.14; }"#)
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    if let ClrValue::Double(n) = result {
        assert!((n - 3.14).abs() < 1e-10);
    } else {
        panic!("expected Double, got {result:?}");
    }
}

#[tokio::test]
async fn net_returns_byte_array() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => { return new byte[] { 0, 1, 2, 255 }; }"#,
        )
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::Buffer(vec![0, 1, 2, 255]));
}

#[tokio::test]
async fn net_returns_array() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => { return new object[] { 1, "two", true }; }"#,
        )
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(
        result,
        ClrValue::Array(vec![
            ClrValue::Int32(1),
            ClrValue::String("two".into()),
            ClrValue::Boolean(true),
        ])
    );
}

#[tokio::test]
async fn net_returns_dictionary() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var d = new System.Collections.Generic.Dictionary<string, object>();
                d["name"] = "dotbridge";
                d["version"] = 1;
                return d;
            }"#,
        )
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    if let ClrValue::Object(map) = result {
        assert_eq!(map.get("name"), Some(&ClrValue::String("dotbridge".into())));
        assert_eq!(map.get("version"), Some(&ClrValue::Int32(1)));
    } else {
        panic!("expected Object, got {result:?}");
    }
}
