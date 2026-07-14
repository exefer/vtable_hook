// Minimal end-to-end vtable hooking example.
// Simulates a C++ object (Itanium ABI), then hooks its virtual method.

use std::ffi::c_void;
use vtable_hook::{RawVTable, Method, hook::copy::raw::RawHook};

// --- Simulate a C++ object ---

type VirtualFn = unsafe extern "C" fn(this: *mut c_void) -> i32;

// The vtable - a null-terminated array of function pointers.
// In real C++ this lives in .data.rel.ro (read-only after relocation).
// We use a static array so it has a stable address.
static VTABLE: [VirtualFn; 2] = [original_fn, null_fn];

unsafe extern "C" fn original_fn(_this: *mut c_void) -> i32 { 0 }
unsafe extern "C" fn null_fn(_this: *mut c_void) -> i32 { 0 }

// A C++ object. First field is the vptr (thin pointer), rest is member data.
#[repr(C)]
struct Object {
    vptr: *const [VirtualFn; 2],
    _value: i32,
}

fn make_object() -> Box<Object> {
    Box::new(Object { vptr: &VTABLE, _value: 42 })
}

// --- Hook ---

static ORIG: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn my_hook(this: *mut c_void) -> i32 {
    unsafe {
        eprintln!("[hook] intercepted call on obj={this:p}");
        let orig: VirtualFn = std::mem::transmute(ORIG.load(std::sync::atomic::Ordering::Relaxed));
        orig(this)
    }
}

fn main() {
    unsafe {
        let mut obj = make_object();
        let obj_ptr: *mut c_void = &mut *obj as *mut Object as *mut c_void;

        // Read a vtable entry and call it.
        let call_slot = |idx: usize| -> i32 {
            let vtable_ptr = obj.vptr as *const VirtualFn;
            let f = *vtable_ptr.add(idx);
            f(obj_ptr)
        };

        eprintln!("before: {}", call_slot(0));

        // Hook it
        let vptr_field = obj_ptr as *mut RawVTable;
        let mut hook = RawHook::new(vptr_field, None);

        // Save original address for forwarding
        let vtable_ptr: RawVTable = *vptr_field;
        let orig_addr = *vtable_ptr.add(0) as usize;
        ORIG.store(orig_addr, std::sync::atomic::Ordering::Relaxed);

        hook.replace_method(0, my_hook as Method);
        hook.enable();

        eprintln!("after:  {}", call_slot(0));

        hook.disable();
        eprintln!("after disable: {}", call_slot(0));
    }
}
