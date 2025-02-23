//! Tests for inline C# compilation — equivalent to edge-js 104_csx.js

mod common;

use dotbridge::ClrValue;

// =============================================================================
// Lambda expression compilation
// =============================================================================

#[tokio::test]
async fn compile_lambda_literal() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return "lambda works"; }"#)
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::String("lambda works".into()));
}

#[tokio::test]
async fn compile_lambda_with_computation() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                int x = (int)input;
                return x * x;
            }"#,
        )
        .unwrap();
    let result = func.call(ClrValue::Int32(7)).await.unwrap();
    assert_eq!(result, ClrValue::Int32(49));
}

#[tokio::test]
async fn compile_lambda_with_string_concat() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                string name = (string)input;
                return $"Hello, {name}!";
            }"#,
        )
        .unwrap();
    let result = func
        .call(ClrValue::String("World".into()))
        .await
        .unwrap();
    assert_eq!(result, ClrValue::String("Hello, World!".into()));
}

#[tokio::test]
async fn compile_lambda_multiline() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                int a = 10;
                int b = 20;
                int c = a + b;
                return c;
            }"#,
        )
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::Int32(30));
}

#[tokio::test]
async fn compile_lambda_with_using_statement() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var sb = new System.Text.StringBuilder();
                sb.Append("Hello ");
                sb.Append("from ");
                sb.Append("StringBuilder");
                return sb.ToString();
            }"#,
        )
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(
        result,
        ClrValue::String("Hello from StringBuilder".into())
    );
}

// =============================================================================
// Class-based compilation
// =============================================================================

#[tokio::test]
async fn compile_class_with_startup_invoke() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"
            using System.Threading.Tasks;
            public class Startup {
                public async Task<object> Invoke(object input) {
                    return "class works";
                }
            }
            "#,
        )
        .unwrap();
    let result = func.call(ClrValue::Null).await.unwrap();
    assert_eq!(result, ClrValue::String("class works".into()));
}

#[tokio::test]
async fn compile_class_with_state() {
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

    // Each call should increment the counter
    let r1 = func.call(ClrValue::Null).await.unwrap();
    let r2 = func.call(ClrValue::Null).await.unwrap();
    let r3 = func.call(ClrValue::Null).await.unwrap();

    assert_eq!(r1, ClrValue::Int32(1));
    assert_eq!(r2, ClrValue::Int32(2));
    assert_eq!(r3, ClrValue::Int32(3));
}

// =============================================================================
// Compilation error handling
// =============================================================================

#[test]
fn compile_malformed_lambda_fails() {
    let runtime = common::init_runtime();
    let result = runtime.func_from_source(r#"async input => return"#);
    assert!(result.is_err());
}

#[test]
fn compile_malformed_class_fails() {
    let runtime = common::init_runtime();
    let result = runtime.func_from_source(
        r#"
        public class {
            invalid syntax here
        }
        "#,
    );
    assert!(result.is_err());
}

#[test]
fn compile_empty_source_fails() {
    let runtime = common::init_runtime();
    let result = runtime.func_from_source("");
    assert!(result.is_err());
}

// =============================================================================
// Dynamic input tests
// =============================================================================

#[tokio::test]
async fn compile_lambda_dynamic_input_object() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var dict = (System.Collections.Generic.IDictionary<string, object>)input;
                string name = (string)dict["name"];
                int value = (int)dict["value"];
                return $"{name}={value}";
            }"#,
        )
        .unwrap();

    let mut map = std::collections::HashMap::new();
    map.insert("name".to_string(), ClrValue::String("x".into()));
    map.insert("value".to_string(), ClrValue::Int32(42));

    let result = func.call(ClrValue::Object(map)).await.unwrap();
    assert_eq!(result, ClrValue::String("x=42".into()));
}

#[tokio::test]
async fn compile_lambda_with_array_input() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var arr = (object[])input;
                return arr.Length;
            }"#,
        )
        .unwrap();

    let arr = ClrValue::Array(vec![
        ClrValue::Int32(1),
        ClrValue::Int32(2),
        ClrValue::Int32(3),
    ]);
    let result = func.call(arr).await.unwrap();
    assert_eq!(result, ClrValue::Int32(3));
}

// =============================================================================
// Multiple functions from same runtime
// =============================================================================

#[tokio::test]
async fn multiple_functions_from_same_runtime() {
    let runtime = common::init_runtime();

    let add = runtime
        .func_from_source(r#"async (input) => { return (int)input + 1; }"#)
        .unwrap();
    let mul = runtime
        .func_from_source(r#"async (input) => { return (int)input * 2; }"#)
        .unwrap();
    let greet = runtime
        .func_from_source(r#"async (input) => { return "Hi " + input; }"#)
        .unwrap();

    assert_eq!(add.call(ClrValue::Int32(5)).await.unwrap(), ClrValue::Int32(6));
    assert_eq!(mul.call(ClrValue::Int32(5)).await.unwrap(), ClrValue::Int32(10));
    assert_eq!(
        greet.call(ClrValue::String("Rust".into())).await.unwrap(),
        ClrValue::String("Hi Rust".into())
    );
}
