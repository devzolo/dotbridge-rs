//! Tests for Rust<->NET callbacks — equivalent to edge-js 103_net2node.js

mod common;

use dotbridge::{ClrValue, DotBridgeError};
use dotbridge::runtime::{register_callback, unregister_callback, invoke_callback};

// =============================================================================
// Callback registry unit tests
// =============================================================================

#[test]
fn register_and_invoke_callback() {
    let id = register_callback(|input| {
        Ok(ClrValue::String(format!("got: {:?}", input)))
    });
    assert!(id > 0);

    let result = invoke_callback(id, ClrValue::Int32(42)).unwrap();
    if let ClrValue::String(s) = result {
        assert!(s.contains("42"));
    } else {
        panic!("expected String, got {result:?}");
    }

    unregister_callback(id);
}

#[test]
fn invoke_unregistered_callback_fails() {
    let result = invoke_callback(999_999, ClrValue::Null);
    assert!(result.is_err());
    match result.unwrap_err() {
        DotBridgeError::CallbackError(msg) => {
            assert!(msg.contains("not found"), "got: {msg}");
        }
        other => panic!("expected CallbackError, got {other:?}"),
    }
}

#[test]
fn register_multiple_callbacks() {
    let id1 = register_callback(|_| Ok(ClrValue::String("cb1".into())));
    let id2 = register_callback(|_| Ok(ClrValue::String("cb2".into())));
    let id3 = register_callback(|_| Ok(ClrValue::String("cb3".into())));

    assert_ne!(id1, id2);
    assert_ne!(id2, id3);

    assert_eq!(
        invoke_callback(id1, ClrValue::Null).unwrap(),
        ClrValue::String("cb1".into())
    );
    assert_eq!(
        invoke_callback(id2, ClrValue::Null).unwrap(),
        ClrValue::String("cb2".into())
    );
    assert_eq!(
        invoke_callback(id3, ClrValue::Null).unwrap(),
        ClrValue::String("cb3".into())
    );

    unregister_callback(id1);
    unregister_callback(id2);
    unregister_callback(id3);
}

#[test]
fn unregister_callback_makes_it_unavailable() {
    let id = register_callback(|_| Ok(ClrValue::Null));
    assert!(invoke_callback(id, ClrValue::Null).is_ok());

    unregister_callback(id);
    assert!(invoke_callback(id, ClrValue::Null).is_err());
}

#[test]
fn callback_can_return_error() {
    let id = register_callback(|_| {
        Err(DotBridgeError::CallbackError("intentional error".into()))
    });

    let result = invoke_callback(id, ClrValue::Null);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("intentional error"));

    unregister_callback(id);
}

#[test]
fn callback_receives_correct_input() {
    use std::sync::{Arc, Mutex};

    let received = Arc::new(Mutex::new(None));
    let received_clone = received.clone();

    let id = register_callback(move |input| {
        *received_clone.lock().unwrap() = Some(input.clone());
        Ok(ClrValue::Null)
    });

    invoke_callback(id, ClrValue::String("test input".into())).unwrap();

    let captured = received.lock().unwrap().take().unwrap();
    assert_eq!(captured, ClrValue::String("test input".into()));

    unregister_callback(id);
}

// =============================================================================
// Callback via runtime API
// =============================================================================

#[test]
fn runtime_register_unregister_callback() {
    let runtime = common::init_runtime();

    let id = runtime.register_callback(|input| {
        Ok(ClrValue::String(format!("runtime cb: {:?}", input)))
    });

    let result = invoke_callback(id, ClrValue::Int32(7)).unwrap();
    if let ClrValue::String(s) = result {
        assert!(s.contains("7"));
    } else {
        panic!("expected String");
    }

    runtime.unregister_callback(id);
    assert!(invoke_callback(id, ClrValue::Null).is_err());
}

// =============================================================================
// Callback with complex data
// =============================================================================

#[test]
fn callback_with_complex_input() {
    use std::collections::HashMap;

    let id = register_callback(|input| {
        if let ClrValue::Object(map) = input {
            let name = match map.get("name") {
                Some(ClrValue::String(s)) => s.clone(),
                _ => "unknown".to_string(),
            };
            Ok(ClrValue::String(format!("Hello, {}!", name)))
        } else {
            Ok(ClrValue::String("not an object".into()))
        }
    });

    let mut map = HashMap::new();
    map.insert("name".to_string(), ClrValue::String("World".into()));

    let result = invoke_callback(id, ClrValue::Object(map)).unwrap();
    assert_eq!(result, ClrValue::String("Hello, World!".into()));

    unregister_callback(id);
}

#[test]
fn callback_returns_complex_data() {
    use std::collections::HashMap;

    let id = register_callback(|_| {
        let mut map = HashMap::new();
        map.insert("status".to_string(), ClrValue::String("ok".into()));
        map.insert("code".to_string(), ClrValue::Int32(200));
        Ok(ClrValue::Object(map))
    });

    let result = invoke_callback(id, ClrValue::Null).unwrap();
    if let ClrValue::Object(map) = result {
        assert_eq!(map.get("status"), Some(&ClrValue::String("ok".into())));
        assert_eq!(map.get("code"), Some(&ClrValue::Int32(200)));
    } else {
        panic!("expected Object");
    }

    unregister_callback(id);
}
