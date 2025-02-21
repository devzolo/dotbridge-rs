use std::collections::HashMap;

use crate::error::DotBridgeError;

/// Represents a value that can be passed between Rust and .NET.
///
/// This mirrors the V8Type enum from edge-js, adapted for Rust types.
/// Each variant maps to a corresponding CLR type during marshaling.
#[derive(Debug, Clone, PartialEq)]
pub enum ClrValue {
    /// .NET null
    Null,
    /// System.String
    String(String),
    /// System.Boolean
    Boolean(bool),
    /// System.Int32
    Int32(i32),
    /// System.UInt32
    UInt32(u32),
    /// System.Int64
    Int64(i64),
    /// System.Double
    Double(f64),
    /// System.Single
    Float(f32),
    /// System.DateTime (stored as milliseconds since Unix epoch)
    DateTime(i64),
    /// System.Guid (stored as string representation)
    Guid(String),
    /// byte[] — System.Byte[]
    Buffer(Vec<u8>),
    /// Array of CLR values — maps to object[]
    Array(Vec<ClrValue>),
    /// Dictionary — maps to IDictionary<string, object>
    Object(HashMap<String, ClrValue>),
    /// A Rust callback function that .NET can invoke.
    /// Stored as a boxed closure.
    Callback(CallbackHandle),
    /// System.Decimal (stored as string for lossless transport)
    Decimal(String),
}

/// Handle to a Rust callback that can be invoked from .NET.
#[derive(Debug, Clone)]
pub struct CallbackHandle {
    id: u64,
}

impl PartialEq for CallbackHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl CallbackHandle {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

/// V8Type-equivalent enum for wire protocol.
/// Used in the binary serialization format between Rust and .NET.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClrTypeTag {
    Function = 1,
    Buffer = 2,
    Array = 3,
    Date = 4,
    Object = 5,
    String = 6,
    Boolean = 7,
    Int32 = 8,
    UInt32 = 9,
    Number = 10,
    Null = 11,
    Task = 12,
    Exception = 13,
}

/// Trait for types that can be converted to a CLR-compatible value.
pub trait ToClrValue {
    fn to_clr_value(&self) -> ClrValue;
}

/// Trait for types that can be constructed from a CLR value.
pub trait FromClrValue: Sized {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError>;
}

// -- ToClrValue implementations for standard types --

impl ToClrValue for () {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Null
    }
}

impl ToClrValue for String {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::String(self.clone())
    }
}

impl ToClrValue for &str {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::String(self.to_string())
    }
}

impl ToClrValue for bool {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Boolean(*self)
    }
}

impl ToClrValue for i32 {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Int32(*self)
    }
}

impl ToClrValue for u32 {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::UInt32(*self)
    }
}

impl ToClrValue for i64 {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Int64(*self)
    }
}

impl ToClrValue for f64 {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Double(*self)
    }
}

impl ToClrValue for f32 {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Float(*self)
    }
}

impl ToClrValue for Vec<u8> {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Buffer(self.clone())
    }
}

impl<T: ToClrValue> ToClrValue for Vec<T> {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Array(self.iter().map(|v| v.to_clr_value()).collect())
    }
}

impl<T: ToClrValue> ToClrValue for Option<T> {
    fn to_clr_value(&self) -> ClrValue {
        match self {
            Some(v) => v.to_clr_value(),
            None => ClrValue::Null,
        }
    }
}

impl<T: ToClrValue> ToClrValue for HashMap<String, T> {
    fn to_clr_value(&self) -> ClrValue {
        ClrValue::Object(
            self.iter()
                .map(|(k, v)| (k.clone(), v.to_clr_value()))
                .collect(),
        )
    }
}

impl ToClrValue for ClrValue {
    fn to_clr_value(&self) -> ClrValue {
        self.clone()
    }
}

// -- FromClrValue implementations for standard types --

impl FromClrValue for () {
    fn from_clr_value(_value: &ClrValue) -> Result<Self, DotBridgeError> {
        Ok(())
    }
}

impl FromClrValue for String {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::String(s) => Ok(s.clone()),
            ClrValue::Null => Ok(String::new()),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected String, got {other:?}"
            ))),
        }
    }
}

impl FromClrValue for bool {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Boolean(b) => Ok(*b),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected Boolean, got {other:?}"
            ))),
        }
    }
}

impl FromClrValue for i32 {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Int32(n) => Ok(*n),
            ClrValue::Double(n) => Ok(*n as i32),
            ClrValue::Int64(n) => Ok(*n as i32),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected Int32, got {other:?}"
            ))),
        }
    }
}

impl FromClrValue for u32 {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::UInt32(n) => Ok(*n),
            ClrValue::Int32(n) => Ok(*n as u32),
            ClrValue::Double(n) => Ok(*n as u32),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected UInt32, got {other:?}"
            ))),
        }
    }
}

impl FromClrValue for i64 {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Int64(n) => Ok(*n),
            ClrValue::Int32(n) => Ok(*n as i64),
            ClrValue::Double(n) => Ok(*n as i64),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected Int64, got {other:?}"
            ))),
        }
    }
}

impl FromClrValue for f64 {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Double(n) => Ok(*n),
            ClrValue::Float(n) => Ok(*n as f64),
            ClrValue::Int32(n) => Ok(*n as f64),
            ClrValue::Int64(n) => Ok(*n as f64),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected Double, got {other:?}"
            ))),
        }
    }
}

impl FromClrValue for f32 {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Float(n) => Ok(*n),
            ClrValue::Double(n) => Ok(*n as f32),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected Float, got {other:?}"
            ))),
        }
    }
}

impl FromClrValue for Vec<u8> {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Buffer(b) => Ok(b.clone()),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected Buffer, got {other:?}"
            ))),
        }
    }
}

impl<T: FromClrValue> FromClrValue for Vec<T> {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Array(arr) => arr.iter().map(T::from_clr_value).collect(),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected Array, got {other:?}"
            ))),
        }
    }
}

impl<T: FromClrValue> FromClrValue for Option<T> {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Null => Ok(None),
            other => T::from_clr_value(other).map(Some),
        }
    }
}

impl<T: FromClrValue> FromClrValue for HashMap<String, T> {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        match value {
            ClrValue::Object(map) => map
                .iter()
                .map(|(k, v)| Ok((k.clone(), T::from_clr_value(v)?)))
                .collect(),
            other => Err(DotBridgeError::MarshalError(format!(
                "expected Object, got {other:?}"
            ))),
        }
    }
}

impl FromClrValue for ClrValue {
    fn from_clr_value(value: &ClrValue) -> Result<Self, DotBridgeError> {
        Ok(value.clone())
    }
}

// -- Binary serialization for wire protocol --
// Used to pass data between Rust native code and the .NET managed bootstrap.

impl ClrValue {
    /// Serialize this value into the binary wire format used by the edge bootstrap.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.write_to(&mut buf);
        buf
    }

    fn write_to(&self, buf: &mut Vec<u8>) {
        match self {
            ClrValue::Null => {
                buf.extend_from_slice(&(ClrTypeTag::Null as i32).to_le_bytes());
            }
            ClrValue::String(s) => {
                buf.extend_from_slice(&(ClrTypeTag::String as i32).to_le_bytes());
                let bytes = s.as_bytes();
                buf.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
            ClrValue::Boolean(b) => {
                buf.extend_from_slice(&(ClrTypeTag::Boolean as i32).to_le_bytes());
                buf.push(if *b { 1 } else { 0 });
            }
            ClrValue::Int32(n) => {
                buf.extend_from_slice(&(ClrTypeTag::Int32 as i32).to_le_bytes());
                buf.extend_from_slice(&n.to_le_bytes());
            }
            ClrValue::UInt32(n) => {
                buf.extend_from_slice(&(ClrTypeTag::UInt32 as i32).to_le_bytes());
                buf.extend_from_slice(&n.to_le_bytes());
            }
            ClrValue::Int64(n) | ClrValue::DateTime(n) => {
                let tag = if matches!(self, ClrValue::DateTime(_)) {
                    ClrTypeTag::Date
                } else {
                    ClrTypeTag::Number
                };
                buf.extend_from_slice(&(tag as i32).to_le_bytes());
                buf.extend_from_slice(&(*n as f64).to_le_bytes());
            }
            ClrValue::Double(n) => {
                buf.extend_from_slice(&(ClrTypeTag::Number as i32).to_le_bytes());
                buf.extend_from_slice(&n.to_le_bytes());
            }
            ClrValue::Float(n) => {
                buf.extend_from_slice(&(ClrTypeTag::Number as i32).to_le_bytes());
                buf.extend_from_slice(&(*n as f64).to_le_bytes());
            }
            ClrValue::Buffer(data) => {
                buf.extend_from_slice(&(ClrTypeTag::Buffer as i32).to_le_bytes());
                buf.extend_from_slice(&(data.len() as i32).to_le_bytes());
                buf.extend_from_slice(data);
            }
            ClrValue::Array(items) => {
                buf.extend_from_slice(&(ClrTypeTag::Array as i32).to_le_bytes());
                buf.extend_from_slice(&(items.len() as i32).to_le_bytes());
                for item in items {
                    item.write_to(buf);
                }
            }
            ClrValue::Object(map) => {
                buf.extend_from_slice(&(ClrTypeTag::Object as i32).to_le_bytes());
                buf.extend_from_slice(&(map.len() as i32).to_le_bytes());
                for (key, value) in map {
                    let key_bytes = key.as_bytes();
                    buf.extend_from_slice(&(key_bytes.len() as i32).to_le_bytes());
                    buf.extend_from_slice(key_bytes);
                    value.write_to(buf);
                }
            }
            ClrValue::Callback(handle) => {
                buf.extend_from_slice(&(ClrTypeTag::Function as i32).to_le_bytes());
                buf.extend_from_slice(&handle.id().to_le_bytes());
            }
            ClrValue::Guid(s) | ClrValue::Decimal(s) => {
                buf.extend_from_slice(&(ClrTypeTag::String as i32).to_le_bytes());
                let bytes = s.as_bytes();
                buf.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
        }
    }

    /// Deserialize a CLR value from the binary wire format.
    pub fn deserialize(data: &[u8]) -> Result<(ClrValue, usize), DotBridgeError> {
        if data.len() < 4 {
            return Err(DotBridgeError::MarshalError("buffer too short for type tag".into()));
        }

        let tag = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let mut offset = 4;

        let value = match tag {
            t if t == ClrTypeTag::Null as i32 => ClrValue::Null,

            t if t == ClrTypeTag::String as i32 => {
                let len = read_i32(&data[offset..])?;
                offset += 4;
                let s = std::str::from_utf8(&data[offset..offset + len as usize])
                    .map_err(|e| DotBridgeError::MarshalError(format!("invalid UTF-8: {e}")))?;
                offset += len as usize;
                ClrValue::String(s.to_string())
            }

            t if t == ClrTypeTag::Boolean as i32 => {
                let b = data[offset] != 0;
                offset += 1;
                ClrValue::Boolean(b)
            }

            t if t == ClrTypeTag::Int32 as i32 => {
                let n = read_i32(&data[offset..])?;
                offset += 4;
                ClrValue::Int32(n)
            }

            t if t == ClrTypeTag::UInt32 as i32 => {
                let n = u32::from_le_bytes([
                    data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                ]);
                offset += 4;
                ClrValue::UInt32(n)
            }

            t if t == ClrTypeTag::Number as i32 => {
                let n = f64::from_le_bytes([
                    data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
                ]);
                offset += 8;
                ClrValue::Double(n)
            }

            t if t == ClrTypeTag::Date as i32 => {
                let n = f64::from_le_bytes([
                    data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
                ]);
                offset += 8;
                ClrValue::DateTime(n as i64)
            }

            t if t == ClrTypeTag::Buffer as i32 => {
                let len = read_i32(&data[offset..])? as usize;
                offset += 4;
                let buf = data[offset..offset + len].to_vec();
                offset += len;
                ClrValue::Buffer(buf)
            }

            t if t == ClrTypeTag::Array as i32 => {
                let count = read_i32(&data[offset..])? as usize;
                offset += 4;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    let (val, consumed) = ClrValue::deserialize(&data[offset..])?;
                    offset += consumed;
                    items.push(val);
                }
                ClrValue::Array(items)
            }

            t if t == ClrTypeTag::Object as i32 => {
                let count = read_i32(&data[offset..])? as usize;
                offset += 4;
                let mut map = HashMap::with_capacity(count);
                for _ in 0..count {
                    let key_len = read_i32(&data[offset..])? as usize;
                    offset += 4;
                    let key = std::str::from_utf8(&data[offset..offset + key_len])
                        .map_err(|e| DotBridgeError::MarshalError(format!("invalid UTF-8 key: {e}")))?
                        .to_string();
                    offset += key_len;
                    let (val, consumed) = ClrValue::deserialize(&data[offset..])?;
                    offset += consumed;
                    map.insert(key, val);
                }
                ClrValue::Object(map)
            }

            t if t == ClrTypeTag::Function as i32 => {
                let id = u64::from_le_bytes([
                    data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
                ]);
                offset += 8;
                ClrValue::Callback(CallbackHandle::new(id))
            }

            t if t == ClrTypeTag::Exception as i32 => {
                let len = read_i32(&data[offset..])? as usize;
                offset += 4;
                let msg = std::str::from_utf8(&data[offset..offset + len])
                    .map_err(|e| DotBridgeError::MarshalError(format!("invalid UTF-8: {e}")))?
                    .to_string();
                return Err(DotBridgeError::DotNetException {
                    message: msg,
                    stack_trace: None,
                });
            }

            _ => {
                return Err(DotBridgeError::MarshalError(format!("unknown type tag: {tag}")));
            }
        };

        Ok((value, offset))
    }
}

fn read_i32(data: &[u8]) -> Result<i32, DotBridgeError> {
    if data.len() < 4 {
        return Err(DotBridgeError::MarshalError("buffer too short for i32".into()));
    }
    Ok(i32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Serialization roundtrip tests
    // =========================================================================

    #[test]
    fn roundtrip_null() {
        let val = ClrValue::Null;
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_string() {
        let val = ClrValue::String("hello world".into());
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_empty_string() {
        let val = ClrValue::String(String::new());
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_unicode_string() {
        let val = ClrValue::String("こんにちは世界 🌍 测试 ñ ü ö".into());
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_long_string() {
        let val = ClrValue::String("a".repeat(100_000));
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_i32() {
        let val = ClrValue::Int32(42);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_i32_negative() {
        let val = ClrValue::Int32(-42);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_i32_boundaries() {
        for &n in &[i32::MIN, i32::MAX, 0, -1, 1] {
            let val = ClrValue::Int32(n);
            let bytes = val.serialize();
            let (result, _) = ClrValue::deserialize(&bytes).unwrap();
            assert_eq!(val, result, "failed for i32 value {n}");
        }
    }

    #[test]
    fn roundtrip_u32() {
        let val = ClrValue::UInt32(42);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_u32_max() {
        let val = ClrValue::UInt32(u32::MAX);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_double() {
        let val = ClrValue::Double(3.14);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_double_special_values() {
        // Zero
        let val = ClrValue::Double(0.0);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);

        // Negative zero
        let val = ClrValue::Double(-0.0);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        if let ClrValue::Double(n) = result {
            assert!(n.is_sign_negative() && n == 0.0);
        } else {
            panic!("expected Double");
        }

        // Infinity
        let val = ClrValue::Double(f64::INFINITY);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);

        // Negative infinity
        let val = ClrValue::Double(f64::NEG_INFINITY);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);

        // NaN
        let val = ClrValue::Double(f64::NAN);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        if let ClrValue::Double(n) = result {
            assert!(n.is_nan());
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn roundtrip_float() {
        let val = ClrValue::Float(2.71);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        // Float is serialized as Number (f64), so it comes back as Double
        if let ClrValue::Double(n) = result {
            assert!((n - 2.71f32 as f64).abs() < 1e-6);
        } else {
            panic!("expected Double, got {result:?}");
        }
    }

    #[test]
    fn roundtrip_bool() {
        let val = ClrValue::Boolean(true);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);

        let val = ClrValue::Boolean(false);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_buffer() {
        let val = ClrValue::Buffer(vec![1, 2, 3, 4, 5]);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_empty_buffer() {
        let val = ClrValue::Buffer(vec![]);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_large_buffer() {
        let val = ClrValue::Buffer(vec![0xAB; 65536]);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_datetime() {
        let val = ClrValue::DateTime(1609459200000); // 2021-01-01T00:00:00Z
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(result, ClrValue::DateTime(1609459200000));
    }

    #[test]
    fn roundtrip_guid() {
        let val = ClrValue::Guid("550e8400-e29b-41d4-a716-446655440000".into());
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        // Guid serializes as String
        assert_eq!(
            result,
            ClrValue::String("550e8400-e29b-41d4-a716-446655440000".into())
        );
    }

    #[test]
    fn roundtrip_decimal() {
        let val = ClrValue::Decimal("12345.6789".into());
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        // Decimal serializes as String
        assert_eq!(result, ClrValue::String("12345.6789".into()));
    }

    #[test]
    fn roundtrip_callback() {
        let val = ClrValue::Callback(CallbackHandle::new(42));
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(result, ClrValue::Callback(CallbackHandle::new(42)));
    }

    #[test]
    fn roundtrip_array() {
        let val = ClrValue::Array(vec![
            ClrValue::Int32(1),
            ClrValue::String("two".into()),
            ClrValue::Boolean(true),
        ]);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_empty_array() {
        let val = ClrValue::Array(vec![]);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_nested_array() {
        let val = ClrValue::Array(vec![
            ClrValue::Array(vec![ClrValue::Int32(1), ClrValue::Int32(2)]),
            ClrValue::Array(vec![ClrValue::String("a".into()), ClrValue::String("b".into())]),
        ]);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_object() {
        let mut map = HashMap::new();
        map.insert("name".into(), ClrValue::String("edge-rs".into()));
        map.insert("version".into(), ClrValue::Int32(1));
        let val = ClrValue::Object(map);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_empty_object() {
        let val = ClrValue::Object(HashMap::new());
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_nested_object() {
        let mut inner = HashMap::new();
        inner.insert("x".into(), ClrValue::Int32(10));
        inner.insert("y".into(), ClrValue::Int32(20));

        let mut outer = HashMap::new();
        outer.insert("point".into(), ClrValue::Object(inner));
        outer.insert("label".into(), ClrValue::String("origin".into()));

        let val = ClrValue::Object(outer);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    #[test]
    fn roundtrip_complex_nested_structure() {
        let mut map = HashMap::new();
        map.insert("name".into(), ClrValue::String("test".into()));
        map.insert("count".into(), ClrValue::Int32(42));
        map.insert("active".into(), ClrValue::Boolean(true));
        map.insert("ratio".into(), ClrValue::Double(3.14));
        map.insert("data".into(), ClrValue::Buffer(vec![1, 2, 3]));
        map.insert("tags".into(), ClrValue::Array(vec![
            ClrValue::String("a".into()),
            ClrValue::String("b".into()),
        ]));
        map.insert("nothing".into(), ClrValue::Null);

        let mut nested = HashMap::new();
        nested.insert("inner".into(), ClrValue::String("value".into()));
        map.insert("nested".into(), ClrValue::Object(nested));

        let val = ClrValue::Object(map);
        let bytes = val.serialize();
        let (result, _) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(val, result);
    }

    // =========================================================================
    // ToClrValue / FromClrValue trait tests
    // =========================================================================

    #[test]
    fn to_from_clr_primitives() {
        assert_eq!(42i32.to_clr_value(), ClrValue::Int32(42));
        assert_eq!(i32::from_clr_value(&ClrValue::Int32(42)).unwrap(), 42);

        assert_eq!("hello".to_clr_value(), ClrValue::String("hello".into()));
        assert_eq!(
            String::from_clr_value(&ClrValue::String("hello".into())).unwrap(),
            "hello"
        );

        assert_eq!(true.to_clr_value(), ClrValue::Boolean(true));
        assert_eq!(bool::from_clr_value(&ClrValue::Boolean(true)).unwrap(), true);
    }

    #[test]
    fn to_from_clr_u32() {
        assert_eq!(100u32.to_clr_value(), ClrValue::UInt32(100));
        assert_eq!(u32::from_clr_value(&ClrValue::UInt32(100)).unwrap(), 100);
    }

    #[test]
    fn to_from_clr_i64() {
        assert_eq!(1_000_000_000_000i64.to_clr_value(), ClrValue::Int64(1_000_000_000_000));
        assert_eq!(i64::from_clr_value(&ClrValue::Int64(999)).unwrap(), 999);
    }

    #[test]
    fn to_from_clr_f64() {
        assert_eq!(2.718f64.to_clr_value(), ClrValue::Double(2.718));
        assert_eq!(f64::from_clr_value(&ClrValue::Double(2.718)).unwrap(), 2.718);
    }

    #[test]
    fn to_from_clr_f32() {
        assert_eq!(1.5f32.to_clr_value(), ClrValue::Float(1.5));
        assert_eq!(f32::from_clr_value(&ClrValue::Float(1.5)).unwrap(), 1.5);
    }

    #[test]
    fn to_from_clr_buffer() {
        let buf = vec![10u8, 20, 30];
        assert_eq!(buf.to_clr_value(), ClrValue::Buffer(vec![10, 20, 30]));
        assert_eq!(Vec::<u8>::from_clr_value(&ClrValue::Buffer(vec![10, 20, 30])).unwrap(), vec![10, 20, 30]);
    }

    #[test]
    fn to_from_clr_vec() {
        let v = vec![1i32, 2, 3];
        let clr = v.to_clr_value();
        assert_eq!(clr, ClrValue::Array(vec![
            ClrValue::Int32(1),
            ClrValue::Int32(2),
            ClrValue::Int32(3),
        ]));
        let back = Vec::<i32>::from_clr_value(&clr).unwrap();
        assert_eq!(back, vec![1, 2, 3]);
    }

    #[test]
    fn to_from_clr_option() {
        let some: Option<i32> = Some(42);
        assert_eq!(some.to_clr_value(), ClrValue::Int32(42));

        let none: Option<i32> = None;
        assert_eq!(none.to_clr_value(), ClrValue::Null);

        assert_eq!(Option::<i32>::from_clr_value(&ClrValue::Int32(42)).unwrap(), Some(42));
        assert_eq!(Option::<i32>::from_clr_value(&ClrValue::Null).unwrap(), None);
    }

    #[test]
    fn to_from_clr_hashmap() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), 1i32);
        map.insert("b".to_string(), 2);

        let clr = map.to_clr_value();
        if let ClrValue::Object(ref obj) = clr {
            assert_eq!(obj.get("a"), Some(&ClrValue::Int32(1)));
            assert_eq!(obj.get("b"), Some(&ClrValue::Int32(2)));
        } else {
            panic!("expected Object");
        }

        let back = HashMap::<String, i32>::from_clr_value(&clr).unwrap();
        assert_eq!(back, map);
    }

    #[test]
    fn to_from_clr_unit() {
        assert_eq!(().to_clr_value(), ClrValue::Null);
        assert_eq!(<()>::from_clr_value(&ClrValue::Int32(42)).unwrap(), ());
    }

    #[test]
    fn clrvalue_identity_roundtrip() {
        let val = ClrValue::String("test".into());
        assert_eq!(val.to_clr_value(), val);
        assert_eq!(ClrValue::from_clr_value(&val).unwrap(), val);
    }

    #[test]
    fn string_from_null_returns_empty() {
        assert_eq!(String::from_clr_value(&ClrValue::Null).unwrap(), "");
    }

    // =========================================================================
    // Cross-type FromClrValue coercion tests
    // =========================================================================

    #[test]
    fn i32_from_double() {
        assert_eq!(i32::from_clr_value(&ClrValue::Double(42.0)).unwrap(), 42);
    }

    #[test]
    fn i32_from_i64() {
        assert_eq!(i32::from_clr_value(&ClrValue::Int64(42)).unwrap(), 42);
    }

    #[test]
    fn u32_from_i32() {
        assert_eq!(u32::from_clr_value(&ClrValue::Int32(42)).unwrap(), 42);
    }

    #[test]
    fn u32_from_double() {
        assert_eq!(u32::from_clr_value(&ClrValue::Double(42.0)).unwrap(), 42);
    }

    #[test]
    fn i64_from_i32() {
        assert_eq!(i64::from_clr_value(&ClrValue::Int32(42)).unwrap(), 42);
    }

    #[test]
    fn i64_from_double() {
        assert_eq!(i64::from_clr_value(&ClrValue::Double(42.0)).unwrap(), 42);
    }

    #[test]
    fn f64_from_float() {
        let result = f64::from_clr_value(&ClrValue::Float(1.5)).unwrap();
        assert!((result - 1.5).abs() < 1e-6);
    }

    #[test]
    fn f64_from_i32() {
        assert_eq!(f64::from_clr_value(&ClrValue::Int32(42)).unwrap(), 42.0);
    }

    #[test]
    fn f64_from_i64() {
        assert_eq!(f64::from_clr_value(&ClrValue::Int64(42)).unwrap(), 42.0);
    }

    #[test]
    fn f32_from_double() {
        let result = f32::from_clr_value(&ClrValue::Double(1.5)).unwrap();
        assert!((result - 1.5).abs() < 1e-6);
    }

    // =========================================================================
    // FromClrValue error tests
    // =========================================================================

    #[test]
    fn i32_from_string_fails() {
        assert!(i32::from_clr_value(&ClrValue::String("nope".into())).is_err());
    }

    #[test]
    fn bool_from_int_fails() {
        assert!(bool::from_clr_value(&ClrValue::Int32(1)).is_err());
    }

    #[test]
    fn string_from_int_fails() {
        assert!(String::from_clr_value(&ClrValue::Int32(42)).is_err());
    }

    #[test]
    fn buffer_from_string_fails() {
        assert!(Vec::<u8>::from_clr_value(&ClrValue::String("nope".into())).is_err());
    }

    #[test]
    fn vec_from_object_fails() {
        let mut map = HashMap::new();
        map.insert("a".into(), ClrValue::Int32(1));
        assert!(Vec::<i32>::from_clr_value(&ClrValue::Object(map)).is_err());
    }

    #[test]
    fn hashmap_from_array_fails() {
        let arr = ClrValue::Array(vec![ClrValue::Int32(1)]);
        assert!(HashMap::<String, i32>::from_clr_value(&arr).is_err());
    }

    // =========================================================================
    // Deserialization error tests
    // =========================================================================

    #[test]
    fn deserialize_empty_buffer_fails() {
        assert!(ClrValue::deserialize(&[]).is_err());
    }

    #[test]
    fn deserialize_short_buffer_fails() {
        assert!(ClrValue::deserialize(&[1, 2]).is_err());
    }

    #[test]
    fn deserialize_unknown_tag_fails() {
        let bytes = 255i32.to_le_bytes();
        assert!(ClrValue::deserialize(&bytes).is_err());
    }

    #[test]
    fn deserialize_exception_returns_error() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(ClrTypeTag::Exception as i32).to_le_bytes());
        let msg = b"something went wrong";
        buf.extend_from_slice(&(msg.len() as i32).to_le_bytes());
        buf.extend_from_slice(msg);

        let err = ClrValue::deserialize(&buf).unwrap_err();
        match err {
            DotBridgeError::DotNetException { message, .. } => {
                assert_eq!(message, "something went wrong");
            }
            other => panic!("expected DotNetException, got {other:?}"),
        }
    }

    // =========================================================================
    // Serialization size / offset tests
    // =========================================================================

    #[test]
    fn serialize_null_is_4_bytes() {
        assert_eq!(ClrValue::Null.serialize().len(), 4);
    }

    #[test]
    fn serialize_bool_is_5_bytes() {
        assert_eq!(ClrValue::Boolean(true).serialize().len(), 5);
    }

    #[test]
    fn serialize_i32_is_8_bytes() {
        assert_eq!(ClrValue::Int32(0).serialize().len(), 8);
    }

    #[test]
    fn serialize_double_is_12_bytes() {
        assert_eq!(ClrValue::Double(0.0).serialize().len(), 12);
    }

    #[test]
    fn deserialize_returns_correct_offset() {
        let val = ClrValue::String("hello".into());
        let bytes = val.serialize();
        let (_, offset) = ClrValue::deserialize(&bytes).unwrap();
        assert_eq!(offset, bytes.len());
    }

    #[test]
    fn deserialize_multiple_values_sequentially() {
        let v1 = ClrValue::Int32(1);
        let v2 = ClrValue::String("two".into());
        let v3 = ClrValue::Boolean(true);

        let mut buf = v1.serialize();
        buf.extend_from_slice(&v2.serialize());
        buf.extend_from_slice(&v3.serialize());

        let (r1, off1) = ClrValue::deserialize(&buf).unwrap();
        assert_eq!(r1, v1);

        let (r2, off2) = ClrValue::deserialize(&buf[off1..]).unwrap();
        assert_eq!(r2, v2);

        let (r3, _) = ClrValue::deserialize(&buf[off1 + off2..]).unwrap();
        assert_eq!(r3, v3);
    }
}
