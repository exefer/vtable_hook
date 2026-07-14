// Minimal end-to-end vtable hooking example.
// Simulates a C++ object (Itanium ABI), then hooks its virtual method.

use std::ffi::c_void;
use vtable_hook::{RawVTable, Method, hook::copy::raw::RawHook};

type VirtualFn = unsafe extern "C" fn(this: *mut c_void) -> i32;

static VTABLE: [VirtualFn; 2] = [original_fn, null_fn];

unsafe extern "C" fn original_fn(_this: *mut c_void) -> i32 { 0 }
unsafe extern "C" fn null_fn(_this: *mut c_void) -> i32 { 0 }

#[repr(C)]
struct Object {
    vptr: *const [VirtualFn; 2],
    _value: i32,
}

fn make_object() -> Box<Object> {
    Box::new(Object { vptr: &VTABLE, _value: 42 })
}

unsafe extern "C" fn my_hook(this: *mut c_void) -> i32 {
    unsafe {
        eprintln!("[hook] intercepted call on obj={this:p}");
        let orig: VirtualFn = std::mem::transmute(ORIG.load(std::sync::atomic::Ordering::Relaxed));
        orig(this)
    }
}

static ORIG: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn main() {
    unsafe {
        let mut obj = make_object();
        let obj_ptr: *mut c_void = &mut *obj as *mut Object as *mut c_void;

        let call_slot = |idx: usize| -> i32 {
            let vtable_ptr = obj.vptr as *const VirtualFn;
            let f = *vtable_ptr.add(idx);
            f(obj_ptr)
        };

        eprintln!("before: {}", call_slot(0));

        let vptr_field = obj_ptr as *mut RawVTable;
        let mut hook = RawHook::new(vptr_field, None);

        // get_original() eliminates the manual ORIG static pattern
        let orig = hook.get_original(0).unwrap();
        ORIG.store(orig as usize, std::sync::atomic::Ordering::Relaxed);

        // hook() = replace_method + enable in one call
        hook.hook(0, my_hook as Method);

        eprintln!("after:  {}", call_slot(0));

        // reset() = restore_all + disable in one call
        hook.reset();
        eprintln!("after reset: {}", call_slot(0));
    }
}
