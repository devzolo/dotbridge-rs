//! Tests for the DotNetMarshal derive macro

use std::collections::HashMap;
use dotbridge::{ClrValue, DotNetMarshal, ToClrValue, FromClrValue};

#[derive(Debug, PartialEq, DotNetMarshal)]
struct SimpleStruct {
    name: String,
    value: i32,
}

#[derive(Debug, PartialEq, DotNetMarshal)]
struct AllTypes {
    s: String,
    i: i32,
    u: u32,
    l: i64,
    d: f64,
    f: f32,
    b: bool,
}

#[derive(Debug, PartialEq, DotNetMarshal)]
struct WithOptional {
    required: String,
    optional: Option<i32>,
}

#[derive(Debug, PartialEq, DotNetMarshal)]
struct WithVec {
    items: Vec<i32>,
    labels: Vec<String>,
}

#[derive(Debug, PartialEq, DotNetMarshal)]
struct Nested {
    point: SimpleStruct,
    label: String,
}

// =============================================================================
// ToClrValue tests
// =============================================================================

#[test]
fn derive_simple_to_clr() {
    let s = SimpleStruct {
        name: "test".into(),
        value: 42,
    };
    let clr = s.to_clr_value();
    if let ClrValue::Object(map) = clr {
        assert_eq!(map.get("name"), Some(&ClrValue::String("test".into())));
        assert_eq!(map.get("value"), Some(&ClrValue::Int32(42)));
    } else {
        panic!("expected Object, got {clr:?}");
    }
}

#[test]
fn derive_all_types_to_clr() {
    let val = AllTypes {
        s: "hello".into(),
        i: -1,
        u: 100,
        l: 999_999_999_999,
        d: 3.14,
        f: 2.71,
        b: true,
    };
    let clr = val.to_clr_value();
    if let ClrValue::Object(map) = clr {
        assert_eq!(map.get("s"), Some(&ClrValue::String("hello".into())));
        assert_eq!(map.get("i"), Some(&ClrValue::Int32(-1)));
        assert_eq!(map.get("u"), Some(&ClrValue::UInt32(100)));
        assert_eq!(map.get("l"), Some(&ClrValue::Int64(999_999_999_999)));
        assert_eq!(map.get("d"), Some(&ClrValue::Double(3.14)));
        assert_eq!(map.get("f"), Some(&ClrValue::Float(2.71)));
        assert_eq!(map.get("b"), Some(&ClrValue::Boolean(true)));
    } else {
        panic!("expected Object");
    }
}

#[test]
fn derive_with_optional_some() {
    let val = WithOptional {
        required: "yes".into(),
        optional: Some(42),
    };
    let clr = val.to_clr_value();
    if let ClrValue::Object(map) = clr {
        assert_eq!(map.get("required"), Some(&ClrValue::String("yes".into())));
        assert_eq!(map.get("optional"), Some(&ClrValue::Int32(42)));
    } else {
        panic!("expected Object");
    }
}

#[test]
fn derive_with_optional_none() {
    let val = WithOptional {
        required: "yes".into(),
        optional: None,
    };
    let clr = val.to_clr_value();
    if let ClrValue::Object(map) = clr {
        assert_eq!(map.get("optional"), Some(&ClrValue::Null));
    } else {
        panic!("expected Object");
    }
}

#[test]
fn derive_with_vec_to_clr() {
    let val = WithVec {
        items: vec![1, 2, 3],
        labels: vec!["a".into(), "b".into()],
    };
    let clr = val.to_clr_value();
    if let ClrValue::Object(map) = clr {
        assert_eq!(
            map.get("items"),
            Some(&ClrValue::Array(vec![
                ClrValue::Int32(1),
                ClrValue::Int32(2),
                ClrValue::Int32(3),
            ]))
        );
        assert_eq!(
            map.get("labels"),
            Some(&ClrValue::Array(vec![
                ClrValue::String("a".into()),
                ClrValue::String("b".into()),
            ]))
        );
    } else {
        panic!("expected Object");
    }
}

// =============================================================================
// FromClrValue tests
// =============================================================================

#[test]
fn derive_simple_from_clr() {
    let mut map = HashMap::new();
    map.insert("name".into(), ClrValue::String("test".into()));
    map.insert("value".into(), ClrValue::Int32(42));

    let result = SimpleStruct::from_clr_value(&ClrValue::Object(map)).unwrap();
    assert_eq!(result, SimpleStruct {
        name: "test".into(),
        value: 42,
    });
}

#[test]
fn derive_from_clr_missing_field_fails() {
    let mut map = HashMap::new();
    map.insert("name".into(), ClrValue::String("test".into()));
    // missing "value"

    let result = SimpleStruct::from_clr_value(&ClrValue::Object(map));
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("value"), "error should mention missing field: {msg}");
}

#[test]
fn derive_from_clr_wrong_type_fails() {
    let result = SimpleStruct::from_clr_value(&ClrValue::String("not an object".into()));
    assert!(result.is_err());
}

#[test]
fn derive_from_clr_wrong_field_type_fails() {
    let mut map = HashMap::new();
    map.insert("name".into(), ClrValue::Int32(42)); // should be String
    map.insert("value".into(), ClrValue::Int32(1));

    let result = SimpleStruct::from_clr_value(&ClrValue::Object(map));
    assert!(result.is_err());
}

// =============================================================================
// Roundtrip tests (ToClrValue -> FromClrValue)
// =============================================================================

#[test]
fn derive_roundtrip_simple() {
    let original = SimpleStruct {
        name: "roundtrip".into(),
        value: 99,
    };
    let clr = original.to_clr_value();
    let recovered = SimpleStruct::from_clr_value(&clr).unwrap();
    assert_eq!(original, recovered);
}

#[test]
fn derive_roundtrip_with_optional() {
    let original = WithOptional {
        required: "hello".into(),
        optional: Some(7),
    };
    let clr = original.to_clr_value();
    let recovered = WithOptional::from_clr_value(&clr).unwrap();
    assert_eq!(original, recovered);
}

#[test]
fn derive_roundtrip_with_vec() {
    let original = WithVec {
        items: vec![10, 20, 30],
        labels: vec!["x".into(), "y".into()],
    };
    let clr = original.to_clr_value();
    let recovered = WithVec::from_clr_value(&clr).unwrap();
    assert_eq!(original, recovered);
}

#[test]
fn derive_roundtrip_nested() {
    let original = Nested {
        point: SimpleStruct {
            name: "origin".into(),
            value: 0,
        },
        label: "center".into(),
    };
    let clr = original.to_clr_value();
    let recovered = Nested::from_clr_value(&clr).unwrap();
    assert_eq!(original, recovered);
}

// =============================================================================
// Serialization roundtrip (binary wire format)
// =============================================================================

#[test]
fn derive_serialize_deserialize_roundtrip() {
    let original = SimpleStruct {
        name: "serialize me".into(),
        value: 123,
    };
    let clr = original.to_clr_value();
    let bytes = clr.serialize();
    let (deserialized, _) = ClrValue::deserialize(&bytes).unwrap();
    let recovered = SimpleStruct::from_clr_value(&deserialized).unwrap();
    assert_eq!(original, recovered);
}
