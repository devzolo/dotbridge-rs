use std::os::raw::c_void;

use crate::coreclr_delegates::*;
use crate::error::SysError;
use crate::hostfxr::{self, HostFxr, HostFxrHandle};

/// Represents a loaded CoreCLR runtime with resolved delegate functions.
pub struct CoreClrLoader {
    hostfxr: HostFxr,
    handle: HostFxrHandle,
    load_assembly_and_get_function_pointer: LoadAssemblyAndGetFunctionPointerFn,
    get_function_pointer: Option<GetFunctionPointerFn>,
}

impl CoreClrLoader {
    /// Initialize the CoreCLR runtime from a `.runtimeconfig.json` file.
    ///
    /// This is the primary way to start the .NET runtime. The runtime config file
    /// specifies the framework version and other settings.
    pub fn from_runtime_config(runtime_config_path: &str) -> Result<Self, SysError> {
        let hostfxr_path = hostfxr::find_hostfxr()?;
        let hostfxr = HostFxr::load(&hostfxr_path)?;

        let config_wide = hostfxr::to_wide_string(runtime_config_path);
        let handle = hostfxr.initialize_for_runtime_config(&config_wide, None)?;

        let load_assembly_ptr = hostfxr.get_runtime_delegate(
            handle,
            HostFxrDelegateType::LoadAssemblyAndGetFunctionPointer,
        )?;

        let load_assembly_and_get_function_pointer: LoadAssemblyAndGetFunctionPointerFn =
            unsafe { std::mem::transmute(load_assembly_ptr) };

        let get_function_pointer = hostfxr
            .get_runtime_delegate(handle, HostFxrDelegateType::GetFunctionPointer)
            .ok()
            .map(|ptr| unsafe { std::mem::transmute::<*const c_void, GetFunctionPointerFn>(ptr) });

        Ok(Self {
            hostfxr,
            handle,
            load_assembly_and_get_function_pointer,
            get_function_pointer,
        })
    }

    /// Load an assembly and get a function pointer to a managed method.
    ///
    /// # Arguments
    /// * `assembly_path` - Path to the .NET assembly (.dll)
    /// * `type_name` - Fully qualified type name (e.g., "Namespace.ClassName, AssemblyName")
    /// * `method_name` - Method name to invoke
    /// * `delegate_type_name` - Delegate type name, or `None` for default (UnmanagedCallersOnly)
    pub fn get_function_pointer(
        &self,
        assembly_path: &str,
        type_name: &str,
        method_name: &str,
        delegate_type_name: Option<&str>,
    ) -> Result<*const c_void, SysError> {
        let assembly_wide = hostfxr::to_wide_string(assembly_path);
        let type_wide = hostfxr::to_wide_string(type_name);
        let method_wide = hostfxr::to_wide_string(method_name);

        let delegate_wide = delegate_type_name.map(|name| hostfxr::to_wide_string(name));
        let delegate_ptr = match &delegate_wide {
            Some(v) => v.as_ptr(),
            None => UNMANAGEDCALLERSONLY_METHOD,
        };

        let mut func_ptr: *const c_void = std::ptr::null();

        let status = unsafe {
            (self.load_assembly_and_get_function_pointer)(
                assembly_wide.as_ptr(),
                type_wide.as_ptr(),
                method_wide.as_ptr(),
                delegate_ptr,
                std::ptr::null(),
                &mut func_ptr,
            )
        };

        if status < 0 {
            return Err(SysError::HostFxrError(status));
        }

        Ok(func_ptr)
    }

    /// Get a function pointer for a method without loading a new assembly.
    /// Useful for calling methods in already-loaded assemblies or framework types.
    pub fn get_function_pointer_for_loaded(
        &self,
        type_name: &str,
        method_name: &str,
        delegate_type_name: Option<&str>,
    ) -> Result<*const c_void, SysError> {
        let func = self.get_function_pointer.ok_or_else(|| {
            SysError::DelegateNotAvailable("GetFunctionPointer".into())
        })?;

        let type_wide = hostfxr::to_wide_string(type_name);
        let method_wide = hostfxr::to_wide_string(method_name);

        let delegate_wide = delegate_type_name.map(|name| hostfxr::to_wide_string(name));
        let delegate_ptr = match &delegate_wide {
            Some(v) => v.as_ptr(),
            None => UNMANAGEDCALLERSONLY_METHOD,
        };

        let mut func_ptr: *const c_void = std::ptr::null();

        let status = unsafe {
            func(
                type_wide.as_ptr(),
                method_wide.as_ptr(),
                delegate_ptr,
                std::ptr::null(),
                std::ptr::null(),
                &mut func_ptr,
            )
        };

        if status < 0 {
            return Err(SysError::HostFxrError(status));
        }

        Ok(func_ptr)
    }

    /// Get the raw hostfxr handle for advanced usage.
    pub fn handle(&self) -> HostFxrHandle {
        self.handle
    }

    /// Get a reference to the underlying hostfxr instance.
    pub fn hostfxr(&self) -> &HostFxr {
        &self.hostfxr
    }
}

impl Drop for CoreClrLoader {
    fn drop(&mut self) {
        let _ = self.hostfxr.close(self.handle);
    }
}
