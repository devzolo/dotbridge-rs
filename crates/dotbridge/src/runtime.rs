use std::collections::HashMap;
use std::os::raw::c_void;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

use dotbridge_sys::CoreClrLoader;

use crate::compiler::CSharpCompiler;
use crate::error::DotBridgeError;
use crate::func::{DotBridgeFunc, InvokeFnPtr, FreeFnPtr, FreeHandleFnPtr};
use crate::marshal::ClrValue;

/// Global callback registry for .NET -> Rust callbacks.
static CALLBACK_REGISTRY: OnceLock<Mutex<CallbackRegistry>> = OnceLock::new();
static CALLBACK_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

type RustCallback = Box<dyn Fn(ClrValue) -> Result<ClrValue, DotBridgeError> + Send + Sync>;

struct CallbackRegistry {
    callbacks: HashMap<u64, RustCallback>,
}

impl CallbackRegistry {
    fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
        }
    }
}

/// Register a Rust callback that .NET code can invoke.
pub fn register_callback<F>(callback: F) -> u64
where
    F: Fn(ClrValue) -> Result<ClrValue, DotBridgeError> + Send + Sync + 'static,
{
    let id = CALLBACK_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    let registry = CALLBACK_REGISTRY.get_or_init(|| Mutex::new(CallbackRegistry::new()));
    registry.lock().unwrap().callbacks.insert(id, Box::new(callback));
    id
}

/// Unregister a Rust callback.
pub fn unregister_callback(id: u64) {
    if let Some(registry) = CALLBACK_REGISTRY.get() {
        registry.lock().unwrap().callbacks.remove(&id);
    }
}

/// Invoke a registered Rust callback (called from .NET via FFI).
pub fn invoke_callback(id: u64, input: ClrValue) -> Result<ClrValue, DotBridgeError> {
    let registry = CALLBACK_REGISTRY
        .get()
        .ok_or_else(|| DotBridgeError::CallbackError("callback registry not initialized".into()))?;

    let guard = registry.lock().unwrap();
    let callback = guard
        .callbacks
        .get(&id)
        .ok_or_else(|| DotBridgeError::CallbackError(format!("callback {id} not found")))?;

    callback(input)
}

/// FFI entry point for .NET to invoke Rust callbacks.
///
/// # Safety
/// Called from .NET via UnmanagedCallersOnly delegate.
#[no_mangle]
pub unsafe extern "system" fn dotbridge_invoke_callback(
    payload: *const u8,
    payload_len: i32,
    result_ptr: *mut *mut u8,
    result_len: *mut i32,
) -> i32 {
    let data = std::slice::from_raw_parts(payload, payload_len as usize);

    let response = match deserialize_callback_payload(data) {
        Ok((id, input)) => match invoke_callback(id, input) {
            Ok(result) => result.serialize(),
            Err(e) => {
                let err_msg = format!("{e}");
                ClrValue::String(err_msg).serialize()
            }
        },
        Err(e) => {
            let err_msg = format!("{e}");
            ClrValue::String(err_msg).serialize()
        }
    };

    let boxed = response.into_boxed_slice();
    *result_len = boxed.len() as i32;
    *result_ptr = Box::into_raw(boxed) as *mut u8;

    0
}

/// FFI entry point for .NET to free memory allocated by Rust callbacks.
///
/// # Safety
/// Must be called with a pointer previously returned by dotbridge_invoke_callback.
#[no_mangle]
pub unsafe extern "system" fn dotbridge_free(ptr: *mut u8, len: i32) {
    if !ptr.is_null() {
        let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len as usize));
    }
}

fn deserialize_callback_payload(data: &[u8]) -> Result<(u64, ClrValue), DotBridgeError> {
    if data.len() < 8 {
        return Err(DotBridgeError::MarshalError("callback payload too short".into()));
    }
    let id = u64::from_le_bytes([
        data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
    ]);
    let (value, _) = ClrValue::deserialize(&data[8..])?;
    Ok((id, value))
}

/// Configuration for initializing the DotBridge runtime.
#[derive(Debug, Clone)]
pub struct DotBridgeConfig {
    /// Path to a `.runtimeconfig.json` file.
    /// If not set, a temporary one will be generated.
    pub runtime_config_path: Option<String>,
    /// Target .NET framework version (e.g., "net8.0"). Default: auto-detect.
    pub target_framework: Option<String>,
    /// Path to the bootstrap assembly directory.
    pub bootstrap_dir: Option<String>,
    /// Additional assembly search paths.
    pub assembly_search_paths: Vec<String>,
    /// Enable debug output.
    pub debug: bool,
}

impl Default for DotBridgeConfig {
    fn default() -> Self {
        Self {
            runtime_config_path: None,
            target_framework: None,
            bootstrap_dir: None,
            assembly_search_paths: Vec::new(),
            debug: std::env::var("DOTBRIDGE_DEBUG").is_ok(),
        }
    }
}

/// The DotBridge runtime — manages the .NET CoreCLR lifetime and provides
/// the main API for calling .NET code from Rust.
pub struct DotBridgeRuntime {
    loader: Arc<CoreClrLoader>,
    compiler: CSharpCompiler,
    invoke_fn: InvokeFnPtr,
    free_fn: FreeFnPtr,
    free_handle_fn: FreeHandleFnPtr,
    config: DotBridgeConfig,
    bootstrap_dir: PathBuf,
}

impl DotBridgeRuntime {
    /// Initialize the runtime with default configuration.
    pub fn new() -> Result<Self, DotBridgeError> {
        Self::with_config(DotBridgeConfig::default())
    }

    /// Initialize the runtime with custom configuration.
    pub fn with_config(config: DotBridgeConfig) -> Result<Self, DotBridgeError> {
        let bootstrap_dir = Self::resolve_bootstrap_dir(&config)?;
        let runtime_config = Self::resolve_runtime_config(&config, &bootstrap_dir)?;

        if config.debug {
            eprintln!("[dotbridge] Bootstrap dir: {}", bootstrap_dir.display());
            eprintln!("[dotbridge] Runtime config: {runtime_config}");
        }

        let loader = CoreClrLoader::from_runtime_config(&runtime_config)?;
        let loader = Arc::new(loader);

        let bootstrap_dll = bootstrap_dir.join("DotBridgeBootstrap.dll");
        let bootstrap_path = bootstrap_dll.to_string_lossy();

        if !bootstrap_dll.is_file() {
            return Err(DotBridgeError::CompilationError(format!(
                "DotBridgeBootstrap.dll not found at '{}'. \
                 Build it with: dotnet build crates/dotbridge/dotnet/DotBridgeBootstrap -c Release",
                bootstrap_path
            )));
        }

        let invoke_ptr = loader.get_function_pointer(
            &bootstrap_path,
            "DotBridgeBootstrap.Invoker, DotBridgeBootstrap",
            "Invoke",
            None,
        )?;
        let invoke_fn: InvokeFnPtr = unsafe { std::mem::transmute(invoke_ptr) };

        let free_ptr = loader.get_function_pointer(
            &bootstrap_path,
            "DotBridgeBootstrap.Invoker, DotBridgeBootstrap",
            "Free",
            None,
        )?;
        let free_fn: FreeFnPtr = unsafe { std::mem::transmute(free_ptr) };

        let free_handle_ptr = loader.get_function_pointer(
            &bootstrap_path,
            "DotBridgeBootstrap.Invoker, DotBridgeBootstrap",
            "FreeHandle",
            None,
        )?;
        let free_handle_fn: FreeHandleFnPtr = unsafe { std::mem::transmute(free_handle_ptr) };

        let compiler = CSharpCompiler::new(loader.clone(), &bootstrap_dir, free_fn)?;

        if config.debug {
            eprintln!("[dotbridge] Runtime initialized successfully");
            eprintln!("[dotbridge] Compiler available: {}", compiler.is_available());
        }

        Ok(Self {
            loader,
            compiler,
            invoke_fn,
            free_fn,
            free_handle_fn,
            config,
            bootstrap_dir,
        })
    }

    /// Create a DotBridgeFunc from inline C# source code.
    ///
    /// The source should be a C# lambda expression like:
    /// ```text
    /// async (input) => { return (int)input + 1; }
    /// ```
    ///
    /// Or a full class with a Startup.Invoke method.
    pub fn func_from_source(&self, source: &str) -> Result<DotBridgeFunc, DotBridgeError> {
        let gc_handle = self.compiler.compile(source)?;
        Ok(DotBridgeFunc::new(gc_handle, self.invoke_fn, self.free_fn, self.free_handle_fn))
    }

    /// Create a DotBridgeFunc from inline C# source code with additional assembly references.
    ///
    /// `references` is a slice of paths to additional .NET assemblies that the code depends on.
    pub fn func_from_source_with_references(
        &self,
        source: &str,
        references: &[&str],
    ) -> Result<DotBridgeFunc, DotBridgeError> {
        let refs = references.join(";");
        let gc_handle = self.compiler.compile_with_references(source, Some(&refs))?;
        Ok(DotBridgeFunc::new(gc_handle, self.invoke_fn, self.free_fn, self.free_handle_fn))
    }

    /// Create a DotBridgeFunc from a pre-compiled assembly.
    ///
    /// # Arguments
    /// * `assembly_path` - Path to the .NET assembly (.dll)
    /// * `type_name` - Fully qualified type name (e.g., "Namespace.Startup, AssemblyName")
    /// * `method_name` - Method name (e.g., "Invoke")
    pub fn func_from_assembly(
        &self,
        assembly_path: &str,
        type_name: &str,
        method_name: &str,
    ) -> Result<DotBridgeFunc, DotBridgeError> {
        let bootstrap_dll = self.bootstrap_dir.join("DotBridgeBootstrap.dll");
        let bootstrap_path = bootstrap_dll.to_string_lossy();

        let get_func_ptr = self.loader.get_function_pointer(
            &bootstrap_path,
            "DotBridgeBootstrap.Invoker, DotBridgeBootstrap",
            "GetFunc",
            None,
        )?;

        type GetFuncFn = unsafe extern "system" fn(
            assembly_path: *const u8,
            assembly_path_len: i32,
            type_name: *const u8,
            type_name_len: i32,
            method_name: *const u8,
            method_name_len: i32,
            result: *mut *mut c_void,
            error_ptr: *mut *mut u8,
            error_len: *mut i32,
        ) -> i32;

        let get_func: GetFuncFn = unsafe { std::mem::transmute(get_func_ptr) };

        let asm_bytes = assembly_path.as_bytes();
        let type_bytes = type_name.as_bytes();
        let method_bytes = method_name.as_bytes();

        let mut result: *mut c_void = std::ptr::null_mut();
        let mut error_ptr: *mut u8 = std::ptr::null_mut();
        let mut error_len: i32 = 0;

        let status = unsafe {
            get_func(
                asm_bytes.as_ptr(), asm_bytes.len() as i32,
                type_bytes.as_ptr(), type_bytes.len() as i32,
                method_bytes.as_ptr(), method_bytes.len() as i32,
                &mut result,
                &mut error_ptr, &mut error_len,
            )
        };

        if status < 0 {
            let msg = if !error_ptr.is_null() && error_len > 0 {
                let data = unsafe { std::slice::from_raw_parts(error_ptr, error_len as usize) };
                let s = String::from_utf8_lossy(data).into_owned();
                unsafe { (self.free_fn)(error_ptr, error_len) };
                s
            } else {
                format!("GetFunc failed with status 0x{status:08X}")
            };
            return Err(DotBridgeError::DotNetException { message: msg, stack_trace: None });
        }

        Ok(DotBridgeFunc::new(result, self.invoke_fn, self.free_fn, self.free_handle_fn))
    }

    /// Register a Rust function as a callback that .NET code can invoke.
    pub fn register_callback<F>(&self, callback: F) -> u64
    where
        F: Fn(ClrValue) -> Result<ClrValue, DotBridgeError> + Send + Sync + 'static,
    {
        register_callback(callback)
    }

    /// Unregister a previously registered callback.
    pub fn unregister_callback(&self, id: u64) {
        unregister_callback(id);
    }

    /// Get the bootstrap assembly directory.
    pub fn bootstrap_dir(&self) -> &Path {
        &self.bootstrap_dir
    }

    /// Get the runtime configuration.
    pub fn config(&self) -> &DotBridgeConfig {
        &self.config
    }

    fn resolve_bootstrap_dir(config: &DotBridgeConfig) -> Result<PathBuf, DotBridgeError> {
        // 1. Explicit config
        if let Some(dir) = &config.bootstrap_dir {
            let p = PathBuf::from(dir);
            if p.join("DotBridgeBootstrap.dll").is_file() {
                return Ok(p);
            }
        }

        // 2. Environment variable
        if let Ok(dir) = std::env::var("DOTBRIDGE_BOOTSTRAP_DIR") {
            let p = PathBuf::from(&dir);
            if p.join("DotBridgeBootstrap.dll").is_file() {
                return Ok(p);
            }
        }

        // 3. Relative to the executable (check multiple paths)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                let dir = exe_dir.join("dotbridge-bootstrap");
                if dir.join("DotBridgeBootstrap.dll").is_file() {
                    return Ok(dir);
                }
                if let Some(parent) = exe_dir.parent() {
                    let dir = parent.join("dotbridge-bootstrap");
                    if dir.join("DotBridgeBootstrap.dll").is_file() {
                        return Ok(dir);
                    }
                }
                if exe_dir.join("DotBridgeBootstrap.dll").is_file() {
                    return Ok(exe_dir.to_path_buf());
                }
            }
        }

        // 4. Relative to current working directory
        if let Ok(cwd) = std::env::current_dir() {
            let dir = cwd.join("dotbridge-bootstrap");
            if dir.join("DotBridgeBootstrap.dll").is_file() {
                return Ok(dir);
            }
        }

        // 5. Fallback: temp directory
        let tmp = std::env::temp_dir().join("dotbridge-bootstrap");
        std::fs::create_dir_all(&tmp)?;
        Ok(tmp)
    }

    fn resolve_runtime_config(
        config: &DotBridgeConfig,
        bootstrap_dir: &Path,
    ) -> Result<String, DotBridgeError> {
        // 1. Check for existing runtimeconfig next to bootstrap
        let existing = bootstrap_dir.join("DotBridgeBootstrap.runtimeconfig.json");
        if existing.is_file() {
            return Ok(existing.to_string_lossy().into_owned());
        }

        // 2. Explicit config path
        if let Some(path) = &config.runtime_config_path {
            return Ok(path.clone());
        }

        // 3. Detect installed runtime version
        let version = Self::detect_runtime_version();
        let tfm = config.target_framework.as_deref().unwrap_or("net8.0");

        let config_content = format!(
            r#"{{
  "runtimeOptions": {{
    "tfm": "{tfm}",
    "rollForward": "LatestMinor",
    "framework": {{
      "name": "Microsoft.NETCore.App",
      "version": "{version}"
    }},
    "configProperties": {{
      "System.Runtime.Serialization.EnableUnsafeBinaryFormatterSerialization": false
    }}
  }}
}}"#
        );

        let config_path = bootstrap_dir.join("dotbridge.runtimeconfig.json");
        std::fs::write(&config_path, &config_content)?;

        Ok(config_path.to_string_lossy().into_owned())
    }

    fn detect_runtime_version() -> String {
        if let Ok(output) = std::process::Command::new("dotnet")
            .arg("--list-runtimes")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut best_version = None;
            for line in stdout.lines() {
                if line.starts_with("Microsoft.NETCore.App ") {
                    if let Some(ver) = line.split_whitespace().nth(1) {
                        best_version = Some(ver.to_string());
                    }
                }
            }
            if let Some(v) = best_version {
                return v;
            }
        }
        "8.0.0".to_string()
    }
}
