//! Tests for error handling and exception propagation — equivalent to edge-js error tests

mod common;

use dotbridge::{ClrValue, DotBridgeError};

// =============================================================================
// .NET exception propagation
// =============================================================================

#[tokio::test]
async fn dotnet_exception_propagates_async() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                throw new System.InvalidOperationException("test error from .NET");
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::Null).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("test error from .NET"),
        "error should contain the .NET exception message, got: {msg}"
    );
}

#[test]
fn dotnet_exception_propagates_sync() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                throw new System.ArgumentException("sync error test");
            }"#,
        )
        .unwrap();

    let result = func.call_sync(ClrValue::Null);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("sync error test"),
        "error should contain the .NET exception message, got: {msg}"
    );
}

#[tokio::test]
async fn dotnet_null_reference_exception() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                string s = null;
                return s.Length;
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::Null).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn dotnet_divide_by_zero() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                int x = 10;
                int y = 0;
                return x / y;
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::Null).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn dotnet_index_out_of_range() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                var arr = new int[] { 1, 2, 3 };
                return arr[10];
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::Null).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn dotnet_custom_exception_message() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(
            r#"async (input) => {
                throw new System.Exception("Custom error: " + input?.ToString());
            }"#,
        )
        .unwrap();

    let result = func.call(ClrValue::String("details".into())).await;
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("Custom error: details"),
        "got: {msg}"
    );
}

// =============================================================================
// Compilation error handling
// =============================================================================

#[test]
fn compilation_error_for_invalid_syntax() {
    let runtime = common::init_runtime();
    let result = runtime.func_from_source("not valid c# code at all");
    assert!(result.is_err());
    let err = result.err().unwrap();
    match err {
        DotBridgeError::CompilationError(msg) => {
            assert!(!msg.is_empty(), "compilation error message should not be empty");
        }
        other => panic!("expected CompilationError, got {other:?}"),
    }
}

#[test]
fn compilation_error_for_missing_semicolon() {
    let runtime = common::init_runtime();
    let result = runtime.func_from_source(r#"async (input) => { return 42 }"#);
    assert!(result.is_err());
}

#[test]
fn compilation_error_for_undefined_variable() {
    let runtime = common::init_runtime();
    let result = runtime.func_from_source(
        r#"async (input) => { return undefinedVariable; }"#,
    );
    assert!(result.is_err());
}

// =============================================================================
// Type mismatch at runtime
// =============================================================================

#[tokio::test]
async fn type_cast_error_at_runtime() {
    let runtime = common::init_runtime();
    let func = runtime
        .func_from_source(r#"async (input) => { return (int)input; }"#)
        .unwrap();

    // Pass a string where int is expected
    let result = func.call(ClrValue::String("not a number".into())).await;
    assert!(result.is_err());
}
