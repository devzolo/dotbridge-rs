use std::os::raw::c_void;
use std::path::Path;
use std::sync::Arc;

use dotbridge_sys::CoreClrLoader;

use crate::error::DotBridgeError;
use crate::func::FreeFnPtr;

/// Handles compilation of inline C# source code into callable .NET delegates.
///
/// 1. Source code is sent to the managed bootstrap assembly
/// 2. The bootstrap uses Roslyn (Microsoft.CodeAnalysis) to compile
/// 3. A delegate is returned that can be invoked via FFI
pub struct CSharpCompiler {
    _loader: Arc<CoreClrLoader>,
    compile_fn: Option<CompileFnPtr>,
    free_fn: Option<FreeFnPtr>,
}

type CompileFnPtr = unsafe extern "system" fn(
    source: *const u8,
    source_len: i32,
    references: *const u8,
    references_len: i32,
    result_ptr: *mut *mut c_void,
    error_ptr: *mut *mut u8,
    error_len: *mut i32,
) -> i32;

impl CSharpCompiler {
    /// Create a new compiler instance.
    ///
    /// The bootstrap assembly must be available in the bootstrap directory.
    pub fn new(
        loader: Arc<CoreClrLoader>,
        bootstrap_dir: &Path,
        free_fn: FreeFnPtr,
    ) -> Result<Self, DotBridgeError> {
        let bootstrap_dll = bootstrap_dir.join("DotBridgeBootstrap.dll");

        let compile_fn = if bootstrap_dll.is_file() {
            let ptr = loader.get_function_pointer(
                &bootstrap_dll.to_string_lossy(),
                "DotBridgeBootstrap.Compiler, DotBridgeBootstrap",
                "CompileFunc",
                None,
            );

            match ptr {
                Ok(ptr) => Some(unsafe { std::mem::transmute::<*const c_void, CompileFnPtr>(ptr) }),
                Err(_) => None,
            }
        } else {
            None
        };

        Ok(Self {
            _loader: loader,
            compile_fn,
            free_fn: Some(free_fn),
        })
    }

    /// Compile inline C# source code and return a GCHandle pointer.
    ///
    /// The source can be:
    /// - A lambda expression: `async (input) => { return input; }`
    /// - A full method body
    /// - A complete class with `Startup.Invoke` method
    ///
    /// Optionally pass additional assembly reference paths (semicolon-separated).
    pub fn compile(&self, source: &str) -> Result<*mut c_void, DotBridgeError> {
        self.compile_with_references(source, None)
    }

    /// Compile with additional assembly references.
    ///
    /// `references` is a semicolon-separated list of assembly paths, e.g.:
    /// `"path/to/Foo.dll;path/to/Bar.dll"`
    pub fn compile_with_references(
        &self,
        source: &str,
        references: Option<&str>,
    ) -> Result<*mut c_void, DotBridgeError> {
        let compile_fn = self.compile_fn.ok_or_else(|| {
            DotBridgeError::CompilationError(
                "C# compiler not available. The DotBridgeBootstrap assembly must be compiled first. \
                 Run `dotnet build` in the bootstrap directory, or use `func_from_assembly()` \
                 with pre-compiled assemblies instead."
                    .into(),
            )
        })?;

        let source_bytes = source.as_bytes();
        let (ref_ptr, ref_len) = match references {
            Some(refs) => {
                let bytes = refs.as_bytes();
                (bytes.as_ptr(), bytes.len() as i32)
            }
            None => (std::ptr::null(), 0),
        };

        let mut result_ptr: *mut c_void = std::ptr::null_mut();
        let mut error_ptr: *mut u8 = std::ptr::null_mut();
        let mut error_len: i32 = 0;

        let status = unsafe {
            compile_fn(
                source_bytes.as_ptr(),
                source_bytes.len() as i32,
                ref_ptr,
                ref_len,
                &mut result_ptr,
                &mut error_ptr,
                &mut error_len,
            )
        };

        if status < 0 {
            let error_msg = if !error_ptr.is_null() && error_len > 0 {
                let data = unsafe { std::slice::from_raw_parts(error_ptr, error_len as usize) };
                let msg = String::from_utf8_lossy(data).into_owned();
                // Free the error buffer allocated by C# Marshal.AllocHGlobal
                if let Some(free_fn) = self.free_fn {
                    unsafe { free_fn(error_ptr, error_len) };
                }
                msg
            } else {
                format!("compilation failed with status 0x{status:08X}")
            };
            return Err(DotBridgeError::CompilationError(error_msg));
        }

        if result_ptr.is_null() {
            return Err(DotBridgeError::CompilationError(
                "compilation succeeded but returned null function pointer".into(),
            ));
        }

        Ok(result_ptr)
    }

    /// Check if the C# compiler is available.
    pub fn is_available(&self) -> bool {
        self.compile_fn.is_some()
    }
}
