use std::collections::HashMap;
use std::os::raw::c_void;

use crate::error::DotBridgeError;
use crate::marshal::ClrValue;

/// Function pointer type for Invoker.Invoke [UnmanagedCallersOnly]
pub type InvokeFnPtr = unsafe extern "system" fn(
    func_handle: *mut c_void,
    input_ptr: *const u8,
    input_len: i32,
    result_ptr: *mut *mut u8,
    result_len: *mut i32,
) -> i32;

/// Function pointer type for Invoker.Free [UnmanagedCallersOnly]
pub type FreeFnPtr = unsafe extern "system" fn(
    ptr: *mut u8,
    len: i32,
);

/// Function pointer type for Invoker.FreeHandle [UnmanagedCallersOnly]
pub type FreeHandleFnPtr = unsafe extern "system" fn(
    func_handle: *mut c_void,
);

/// A handle to a .NET function that can be called from Rust.
///
/// Created via `DotBridgeRuntime::func_from_source()` or `DotBridgeRuntime::func_from_assembly()`.
///
/// The underlying .NET method should have the signature:
/// ```text
/// public async Task<object> Invoke(object input)
/// ```
///
/// When dropped, the underlying .NET GCHandle is freed automatically.
///
/// # Examples
/// ```no_run
/// # use dotbridge::{ClrValue, DotBridgeRuntime, DotBridgeError};
/// # async fn example() -> Result<(), DotBridgeError> {
/// # let runtime = DotBridgeRuntime::new()?;
/// let add = runtime.func_from_source(r#"
///     async (input) => { return (int)input + 1; }
/// "#)?;
///
/// // Async call
/// let result = add.call(ClrValue::Int32(5)).await?;
///
/// // Sync call
/// let result = add.call_sync(ClrValue::Int32(5))?;
/// # Ok(())
/// # }
/// ```
pub struct DotBridgeFunc {
    /// GCHandle to the ManagedInvoker on the .NET side.
    gc_handle: *mut c_void,
    /// Pointer to Invoker.Invoke [UnmanagedCallersOnly]
    invoke_fn: InvokeFnPtr,
    /// Pointer to Invoker.Free [UnmanagedCallersOnly]
    free_fn: FreeFnPtr,
    /// Pointer to Invoker.FreeHandle [UnmanagedCallersOnly]
    free_handle_fn: FreeHandleFnPtr,
}

// Safety: The .NET runtime handles thread synchronization internally.
// The GCHandle and function pointers are valid for the lifetime of the runtime.
unsafe impl Send for DotBridgeFunc {}
unsafe impl Sync for DotBridgeFunc {}

impl DotBridgeFunc {
    pub(crate) fn new(
        gc_handle: *mut c_void,
        invoke_fn: InvokeFnPtr,
        free_fn: FreeFnPtr,
        free_handle_fn: FreeHandleFnPtr,
    ) -> Self {
        Self {
            gc_handle,
            invoke_fn,
            free_fn,
            free_handle_fn,
        }
    }

    /// Call the .NET function asynchronously.
    ///
    /// The input is serialized, sent to .NET, and the Task result is awaited.
    pub async fn call(&self, input: ClrValue) -> Result<ClrValue, DotBridgeError> {
        let gc_handle = self.gc_handle as usize;
        let invoke_fn_addr = self.invoke_fn as usize;
        let free_fn_addr = self.free_fn as usize;
        let serialized = input.serialize();

        tokio::task::spawn_blocking(move || {
            let gc_handle = gc_handle as *mut c_void;
            let invoke_fn: InvokeFnPtr = unsafe { std::mem::transmute(invoke_fn_addr) };
            let free_fn: FreeFnPtr = unsafe { std::mem::transmute(free_fn_addr) };
            Self::invoke_raw(gc_handle, invoke_fn, free_fn, &serialized)
        })
        .await
        .map_err(|e| DotBridgeError::TaskFaulted(format!("join error: {e}")))?
    }

    /// Call the .NET function synchronously (blocking).
    pub fn call_sync(&self, input: ClrValue) -> Result<ClrValue, DotBridgeError> {
        let serialized = input.serialize();
        Self::invoke_raw(self.gc_handle, self.invoke_fn, self.free_fn, &serialized)
    }

    /// Raw invocation: call Invoker.Invoke with the GCHandle + serialized input.
    fn invoke_raw(
        gc_handle: *mut c_void,
        invoke_fn: InvokeFnPtr,
        free_fn: FreeFnPtr,
        serialized_input: &[u8],
    ) -> Result<ClrValue, DotBridgeError> {
        let mut result_ptr: *mut u8 = std::ptr::null_mut();
        let mut result_len: i32 = 0;

        let status = unsafe {
            invoke_fn(
                gc_handle,
                serialized_input.as_ptr(),
                serialized_input.len() as i32,
                &mut result_ptr,
                &mut result_len,
            )
        };

        if status < 0 {
            if !result_ptr.is_null() && result_len > 0 {
                let error_data =
                    unsafe { std::slice::from_raw_parts(result_ptr, result_len as usize) };
                let error = Self::parse_structured_error(error_data);
                unsafe { free_fn(result_ptr, result_len) };
                return Err(error);
            }
            return Err(DotBridgeError::DotNetException {
                message: format!("invocation failed with status 0x{status:08X}"),
                stack_trace: None,
            });
        }

        if result_ptr.is_null() || result_len == 0 {
            return Ok(ClrValue::Null);
        }

        let result_data =
            unsafe { std::slice::from_raw_parts(result_ptr, result_len as usize) };
        let (value, _) = ClrValue::deserialize(result_data)?;
        unsafe { free_fn(result_ptr, result_len) };

        Ok(value)
    }

    /// Parse structured exception data from WireProtocol-serialized bytes.
    /// The C# side serializes exceptions as objects with Message, StackTrace, Name fields.
    /// Falls back to raw text if deserialization fails.
    fn parse_structured_error(data: &[u8]) -> DotBridgeError {
        if let Ok((value, _)) = ClrValue::deserialize(data) {
            if let ClrValue::Object(ref map) = value {
                return Self::extract_error_from_object(map);
            }
        }
        // Fallback: treat as raw UTF-8 text
        let message = String::from_utf8_lossy(data).into_owned();
        DotBridgeError::DotNetException {
            message,
            stack_trace: None,
        }
    }

    /// Extract structured error fields from a deserialized exception object.
    fn extract_error_from_object(map: &HashMap<String, ClrValue>) -> DotBridgeError {
        let message = map
            .get("Message")
            .and_then(|v| match v {
                ClrValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "unknown .NET exception".into());

        let stack_trace = map.get("StackTrace").and_then(|v| match v {
            ClrValue::String(s) if !s.is_empty() => Some(s.clone()),
            _ => None,
        });

        let name = map.get("Name").and_then(|v| match v {
            ClrValue::String(s) => Some(s.clone()),
            _ => None,
        });

        // Format: "ExceptionType: message" if we have a name
        let full_message = if let Some(name) = name {
            format!("{name}: {message}")
        } else {
            message
        };

        DotBridgeError::DotNetException {
            message: full_message,
            stack_trace,
        }
    }

    /// Get the raw GCHandle pointer (for advanced usage).
    pub fn as_raw_ptr(&self) -> *mut c_void {
        self.gc_handle
    }
}

impl Drop for DotBridgeFunc {
    fn drop(&mut self) {
        if !self.gc_handle.is_null() {
            unsafe { (self.free_handle_fn)(self.gc_handle) };
        }
    }
}
