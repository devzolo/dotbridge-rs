use std::os::raw::{c_int, c_void};
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};

use crate::coreclr_delegates::{CharT, HostFxrDelegateType};
use crate::error::SysError;

/// Opaque handle returned by hostfxr initialization functions.
pub type HostFxrHandle = *const c_void;

// -- hostfxr function pointer types --

type HostFxrInitializeForRuntimeConfigFn = unsafe extern "system" fn(
    runtime_config_path: *const CharT,
    parameters: *const HostFxrInitializeParameters,
    host_context_handle: *mut HostFxrHandle,
) -> c_int;

type HostFxrInitializeForDotnetCommandLineFn = unsafe extern "system" fn(
    argc: c_int,
    argv: *const *const CharT,
    parameters: *const HostFxrInitializeParameters,
    host_context_handle: *mut HostFxrHandle,
) -> c_int;

type HostFxrGetRuntimeDelegateFn = unsafe extern "system" fn(
    host_context_handle: HostFxrHandle,
    r#type: c_int,
    delegate: *mut *const c_void,
) -> c_int;

type HostFxrCloseFn = unsafe extern "system" fn(host_context_handle: HostFxrHandle) -> c_int;

type HostFxrSetErrorWriterFn =
    unsafe extern "system" fn(error_writer: Option<HostFxrErrorWriterFn>) -> *const c_void;

type HostFxrErrorWriterFn = unsafe extern "system" fn(message: *const CharT);

type HostFxrGetRuntimePropertyValueFn = unsafe extern "system" fn(
    host_context_handle: HostFxrHandle,
    name: *const CharT,
    value: *mut *const CharT,
) -> c_int;

type HostFxrSetRuntimePropertyValueFn = unsafe extern "system" fn(
    host_context_handle: HostFxrHandle,
    name: *const CharT,
    value: *const CharT,
) -> c_int;

/// Parameters for hostfxr initialization.
#[repr(C)]
pub struct HostFxrInitializeParameters {
    pub size: usize,
    pub host_path: *const CharT,
    pub dotnet_root: *const CharT,
}

/// Loaded hostfxr library with resolved function pointers.
pub struct HostFxr {
    _library: Library,
    initialize_for_runtime_config: HostFxrInitializeForRuntimeConfigFn,
    initialize_for_dotnet_command_line: HostFxrInitializeForDotnetCommandLineFn,
    get_runtime_delegate: HostFxrGetRuntimeDelegateFn,
    close: HostFxrCloseFn,
    _set_error_writer: Option<HostFxrSetErrorWriterFn>,
    _get_runtime_property_value: Option<HostFxrGetRuntimePropertyValueFn>,
    set_runtime_property_value: Option<HostFxrSetRuntimePropertyValueFn>,
}

impl HostFxr {
    /// Load hostfxr from a specific path.
    pub fn load(path: &Path) -> Result<Self, SysError> {
        unsafe {
            let library = Library::new(path)?;

            let initialize_for_runtime_config: Symbol<HostFxrInitializeForRuntimeConfigFn> =
                library.get(b"hostfxr_initialize_for_runtime_config\0")?;
            let initialize_for_dotnet_command_line: Symbol<HostFxrInitializeForDotnetCommandLineFn> =
                library.get(b"hostfxr_initialize_for_dotnet_command_line\0")?;
            let get_runtime_delegate: Symbol<HostFxrGetRuntimeDelegateFn> =
                library.get(b"hostfxr_get_runtime_delegate\0")?;
            let close: Symbol<HostFxrCloseFn> = library.get(b"hostfxr_close\0")?;

            let set_error_writer = library
                .get::<HostFxrSetErrorWriterFn>(b"hostfxr_set_error_writer\0")
                .ok()
                .map(|s| *s);
            let get_runtime_property_value = library
                .get::<HostFxrGetRuntimePropertyValueFn>(
                    b"hostfxr_get_runtime_property_value\0",
                )
                .ok()
                .map(|s| *s);
            let set_runtime_property_value = library
                .get::<HostFxrSetRuntimePropertyValueFn>(
                    b"hostfxr_set_runtime_property_value\0",
                )
                .ok()
                .map(|s| *s);

            Ok(Self {
                initialize_for_runtime_config: *initialize_for_runtime_config,
                initialize_for_dotnet_command_line: *initialize_for_dotnet_command_line,
                get_runtime_delegate: *get_runtime_delegate,
                close: *close,
                _set_error_writer: set_error_writer,
                _get_runtime_property_value: get_runtime_property_value,
                set_runtime_property_value,
                _library: library,
            })
        }
    }

    /// Initialize the hosting context from a `.runtimeconfig.json` file.
    pub fn initialize_for_runtime_config(
        &self,
        runtime_config_path: &[CharT],
        parameters: Option<&HostFxrInitializeParameters>,
    ) -> Result<HostFxrHandle, SysError> {
        let mut handle: HostFxrHandle = std::ptr::null();
        let params_ptr = parameters
            .map(|p| p as *const _)
            .unwrap_or(std::ptr::null());

        let status = unsafe {
            (self.initialize_for_runtime_config)(
                runtime_config_path.as_ptr(),
                params_ptr,
                &mut handle,
            )
        };

        // Status 0 = Success, 1 = Success_HostAlreadyInitialized,
        // 2 = Success_DifferentRuntimeProperties
        if status < 0 {
            return Err(SysError::HostFxrError(status));
        }

        Ok(handle)
    }

    /// Initialize the hosting context for a dotnet command line.
    pub fn initialize_for_dotnet_command_line(
        &self,
        args: &[*const CharT],
        parameters: Option<&HostFxrInitializeParameters>,
    ) -> Result<HostFxrHandle, SysError> {
        let mut handle: HostFxrHandle = std::ptr::null();
        let params_ptr = parameters
            .map(|p| p as *const _)
            .unwrap_or(std::ptr::null());

        let status = unsafe {
            (self.initialize_for_dotnet_command_line)(
                args.len() as c_int,
                args.as_ptr(),
                params_ptr,
                &mut handle,
            )
        };

        if status < 0 {
            return Err(SysError::HostFxrError(status));
        }

        Ok(handle)
    }

    /// Get a runtime delegate from the hosting context.
    pub fn get_runtime_delegate(
        &self,
        handle: HostFxrHandle,
        delegate_type: HostFxrDelegateType,
    ) -> Result<*const c_void, SysError> {
        let mut delegate: *const c_void = std::ptr::null();

        let status = unsafe {
            (self.get_runtime_delegate)(handle, delegate_type as c_int, &mut delegate)
        };

        if status < 0 {
            return Err(SysError::HostFxrError(status));
        }

        Ok(delegate)
    }

    /// Close a hosting context handle.
    pub fn close(&self, handle: HostFxrHandle) -> Result<(), SysError> {
        let status = unsafe { (self.close)(handle) };
        if status < 0 {
            return Err(SysError::HostFxrError(status));
        }
        Ok(())
    }

    /// Set a runtime property value.
    pub fn set_runtime_property_value(
        &self,
        handle: HostFxrHandle,
        name: &[CharT],
        value: &[CharT],
    ) -> Result<(), SysError> {
        let func = self
            .set_runtime_property_value
            .ok_or_else(|| SysError::SymbolNotFound {
                name: "hostfxr_set_runtime_property_value".into(),
            })?;

        let status = unsafe { func(handle, name.as_ptr(), value.as_ptr()) };
        if status < 0 {
            return Err(SysError::HostFxrError(status));
        }
        Ok(())
    }
}

// -- Platform-specific helpers for locating hostfxr --

/// Find the hostfxr library on the current system.
pub fn find_hostfxr() -> Result<PathBuf, SysError> {
    let mut searched = Vec::new();

    // 1. Check DOTNET_ROOT environment variable
    if let Ok(dotnet_root) = std::env::var("DOTNET_ROOT") {
        let path = find_hostfxr_in_dotnet_root(Path::new(&dotnet_root));
        if let Some(p) = path {
            return Ok(p);
        }
        searched.push(dotnet_root);
    }

    // 2. Check common installation paths
    for base in platform_dotnet_paths() {
        let path = find_hostfxr_in_dotnet_root(Path::new(&base));
        if let Some(p) = path {
            return Ok(p);
        }
        searched.push(base);
    }

    // 3. Try to find via `dotnet --info` on PATH
    if let Ok(output) = std::process::Command::new("dotnet")
        .arg("--list-runtimes")
        .output()
    {
        if output.status.success() {
            // dotnet is on PATH, try to find root
            if let Ok(output) = std::process::Command::new("dotnet")
                .arg("--info")
                .output()
            {
                let info = String::from_utf8_lossy(&output.stdout);
                for line in info.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Host") || trimmed.contains("hostfxr") {
                        // Parse path from info output
                    }
                    if let Some(rest) = trimmed.strip_prefix("Base Path:") {
                        let base = rest.trim();
                        // Base path is like /usr/share/dotnet/sdk/8.0.100/
                        // dotnet root is 2 levels up
                        if let Some(dotnet_root) =
                            Path::new(base).parent().and_then(|p| p.parent())
                        {
                            let path = find_hostfxr_in_dotnet_root(dotnet_root);
                            if let Some(p) = path {
                                return Ok(p);
                            }
                            searched.push(dotnet_root.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
    }

    Err(SysError::HostFxrNotFound {
        searched_paths: searched,
    })
}

fn find_hostfxr_in_dotnet_root(dotnet_root: &Path) -> Option<PathBuf> {
    let host_dir = dotnet_root.join("host").join("fxr");
    if !host_dir.is_dir() {
        return None;
    }

    // Find the highest version directory
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&host_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();

    versions.sort();
    let latest = versions.last()?;

    let lib_name = hostfxr_lib_name();
    let lib_path = latest.join(lib_name);

    if lib_path.is_file() {
        Some(lib_path)
    } else {
        None
    }
}

#[cfg(windows)]
fn hostfxr_lib_name() -> &'static str {
    "hostfxr.dll"
}

#[cfg(target_os = "macos")]
fn hostfxr_lib_name() -> &'static str {
    "libhostfxr.dylib"
}

#[cfg(target_os = "linux")]
fn hostfxr_lib_name() -> &'static str {
    "libhostfxr.so"
}

fn platform_dotnet_paths() -> Vec<String> {
    let mut paths = Vec::new();

    #[cfg(windows)]
    {
        if let Ok(pf) = std::env::var("ProgramFiles") {
            paths.push(format!("{pf}\\dotnet"));
        }
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            paths.push(format!("{pf86}\\dotnet"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        paths.push("/usr/local/share/dotnet".into());
        paths.push("/opt/homebrew/share/dotnet".into());
    }

    #[cfg(target_os = "linux")]
    {
        paths.push("/usr/share/dotnet".into());
        paths.push("/usr/lib/dotnet".into());
        paths.push("/snap/dotnet-sdk/current".into());
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{home}/.dotnet"));
        }
    }

    paths
}

// -- Wide string helpers --

/// Convert a Rust string to a null-terminated wide string (UTF-16 on Windows).
#[cfg(windows)]
pub fn to_wide_string(s: &str) -> Vec<CharT> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Convert a Rust string to a null-terminated C string (UTF-8 on Unix).
#[cfg(not(windows))]
pub fn to_wide_string(s: &str) -> Vec<CharT> {
    let mut v: Vec<CharT> = s.as_bytes().iter().map(|&b| b as CharT).collect();
    v.push(0);
    v
}

/// Convert a null-terminated wide string back to a Rust String.
#[cfg(windows)]
pub fn from_wide_string(s: *const CharT) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe {
        let len = (0..).take_while(|&i| *s.add(i) != 0).count();
        let slice = std::slice::from_raw_parts(s, len);
        String::from_utf16_lossy(slice)
    }
}

#[cfg(not(windows))]
pub fn from_wide_string(s: *const CharT) -> String {
    if s.is_null() {
        return String::new();
    }
    unsafe {
        std::ffi::CStr::from_ptr(s)
            .to_string_lossy()
            .into_owned()
    }
}
