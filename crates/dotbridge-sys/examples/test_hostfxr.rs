use dotbridge_sys::hostfxr;
use dotbridge_sys::coreclr_delegates::{HostFxrDelegateType, UNMANAGEDCALLERSONLY_METHOD};
use std::os::raw::c_void;

fn main() {
    let path = hostfxr::find_hostfxr().unwrap();
    let fxr = hostfxr::HostFxr::load(&path).unwrap();

    let config = r"C:\Users\devzolo\Desktop\edge-rs\target\debug\dotbridge-bootstrap\DotBridgeBootstrap.runtimeconfig.json";
    println!("Config exists: {}", std::path::Path::new(config).exists());
    let wide = hostfxr::to_wide_string(config);

    let handle = fxr.initialize_for_runtime_config(&wide, None).unwrap();
    println!("Runtime initialized OK");

    let load_asm_ptr = fxr.get_runtime_delegate(
        handle,
        HostFxrDelegateType::LoadAssemblyAndGetFunctionPointer,
    ).unwrap();
    println!("Got LoadAssemblyAndGetFunctionPointer");

    type LoadAsmFn = unsafe extern "system" fn(
        assembly_path: *const u16,
        type_name: *const u16,
        method_name: *const u16,
        delegate_type_name: *const u16,
        reserved: *const c_void,
        delegate: *mut *const c_void,
    ) -> i32;

    let load_asm: LoadAsmFn = unsafe { std::mem::transmute(load_asm_ptr) };

    let assembly = r"C:\Users\devzolo\Desktop\edge-rs\target\debug\dotbridge-bootstrap\DotBridgeBootstrap.dll";
    println!("Assembly: {} (exists: {})", assembly, std::path::Path::new(assembly).exists());

    let asm_w = hostfxr::to_wide_string(assembly);
    let type_w = hostfxr::to_wide_string("DotBridgeBootstrap.Invoker, DotBridgeBootstrap");
    let method_w = hostfxr::to_wide_string("Invoke");
    let mut fp: *const c_void = std::ptr::null();

    println!("\nTest: UnmanagedCallersOnly (UNMANAGEDCALLERSONLY_METHOD sentinel)");
    let status = unsafe {
        load_asm(asm_w.as_ptr(), type_w.as_ptr(), method_w.as_ptr(), UNMANAGEDCALLERSONLY_METHOD, std::ptr::null(), &mut fp)
    };
    println!("  Status: 0x{:08X}", status as u32);
    if status == 0 {
        println!("  Function pointer: {:?}", fp);
    }
}
