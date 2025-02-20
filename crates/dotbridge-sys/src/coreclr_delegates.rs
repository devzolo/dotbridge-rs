#[allow(unused_imports)]
use std::os::raw::{c_char, c_int, c_void};

/// Delegate types that can be requested from hostfxr_get_runtime_delegate.
///
/// These mirror the `hostfxr_delegate_type` enum from the .NET hosting API.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostFxrDelegateType {
    ComActivation = 0,
    LoadInMemoryAssembly = 1,
    WinrtActivation = 2,
    ComRegister = 3,
    ComUnregister = 4,
    LoadAssemblyAndGetFunctionPointer = 5,
    GetFunctionPointer = 6,
    LoadAssembly = 7,
    LoadAssemblyBytes = 8,
}

// -- Wide string type alias for platform interop --
#[cfg(windows)]
pub type CharT = u16;
#[cfg(not(windows))]
pub type CharT = c_char;

/// Function pointer type: load an assembly and get a function pointer from it.
///
/// Signature: `int fn(assembly_path, type_name, method_name, delegate_type_name, reserved, delegate)`
pub type LoadAssemblyAndGetFunctionPointerFn = unsafe extern "system" fn(
    assembly_path: *const CharT,
    type_name: *const CharT,
    method_name: *const CharT,
    delegate_type_name: *const CharT,
    reserved: *const c_void,
    delegate: *mut *const c_void,
) -> c_int;

/// Function pointer type: get a function pointer for a method (no assembly load).
///
/// Signature: `int fn(type_name, method_name, delegate_type_name, load_context, reserved, delegate)`
pub type GetFunctionPointerFn = unsafe extern "system" fn(
    type_name: *const CharT,
    method_name: *const CharT,
    delegate_type_name: *const CharT,
    load_context: *const c_void,
    reserved: *const c_void,
    delegate: *mut *const c_void,
) -> c_int;

/// Function pointer type: load an assembly into the default load context.
pub type LoadAssemblyFn = unsafe extern "system" fn(
    assembly_path: *const CharT,
) -> c_int;

/// Function pointer type: load an assembly from byte arrays.
pub type LoadAssemblyBytesFn = unsafe extern "system" fn(
    assembly: *const u8,
    assembly_size: usize,
    symbols: *const u8,
    symbols_size: usize,
) -> c_int;

/// Sentinel value for `delegate_type_name` parameter to indicate that
/// the target method uses `[UnmanagedCallersOnly]` attribute.
///
/// This is `(const char_t*)-1` in the .NET hosting headers.
/// Note: `std::ptr::null()` means "use default component entry point", NOT UnmanagedCallersOnly.
pub const UNMANAGEDCALLERSONLY_METHOD: *const CharT = usize::MAX as *const CharT;

/// Component entry point delegate: `int fn(void* arg, int arg_size_in_bytes)`
pub type ComponentEntryPointFn =
    unsafe extern "system" fn(arg: *const c_void, arg_size: c_int) -> c_int;

/// Generic managed function delegate: `void* fn(void* arg)`
pub type ManagedFunctionFn = unsafe extern "system" fn(arg: *const c_void) -> *const c_void;

/// Edge bootstrap delegate — called to initialize the managed side.
/// Matches the signature: `void* BootstrapInit(void* context)`
pub type BootstrapInitFn = unsafe extern "system" fn(context: *mut c_void) -> *mut c_void;

/// Edge invoke delegate — called to invoke a managed method.
/// Matches: `void* Invoke(void* payload)`
pub type InvokeFn = unsafe extern "system" fn(payload: *mut c_void) -> *mut c_void;

/// Edge compile delegate — called to compile inline C# source.
/// Matches: `void* CompileFunc(void* payload)`
pub type CompileFn = unsafe extern "system" fn(payload: *mut c_void) -> *mut c_void;
