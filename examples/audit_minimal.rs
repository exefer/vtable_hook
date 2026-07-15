// Minimal end-to-end vtable hooking example.
// Simulates a C++ object (Itanium ABI), then hooks its virtual method.

use std::ffi::c_void;
use std::sync::OnceLock;

use vtable_hook::{RawVTable, hook::copy::raw::RawHook};

type VirtualFn = unsafe extern "C" fn(this: *mut c_void) -> i32;

// Real C++ vtables are null-terminated (Itanium ABI).
// Here the null sentinel happens to be adjacent zeroed memory.
static VTABLE: [VirtualFn; 2] = [original_fn, null_fn];

unsafe extern "C" fn original_fn(_this: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn null_fn(_this: *mut c_void) -> i32 {
    0
}

#[repr(C)]
struct Object {
    vptr: *const [VirtualFn; 2],
    _value: i32,
}

fn make_object() -> Box<Object> {
    Box::new(Object {
        vptr: &VTABLE,
        _value: 42,
    })
}

unsafe extern "C" fn my_hook(this: *mut c_void) -> i32 {
    unsafe {
        eprintln!("[hook] intercepted call on obj={this:p}");
        ORIG.get().unwrap()(this)
    }
}

static ORIG: OnceLock<VirtualFn> = OnceLock::new();

fn main() {
    unsafe {
        let mut obj = make_object();
        let obj_ptr = &raw mut *obj as *mut c_void;

        let call_slot = |idx: usize| -> i32 {
            let vtable_ptr = obj.vptr as *const VirtualFn;
            let f = *vtable_ptr.add(idx);
            f(obj_ptr)
        };

        eprintln!("before: {}", call_slot(0));

        let vptr_field = obj_ptr as *mut RawVTable;
        let mut hook = RawHook::new(vptr_field, None);

        ORIG.set(hook.original_fn::<VirtualFn>(0)).ok();

        // hook() = replace_method + enable in one call
        hook.hook(0, my_hook as *const c_void);

        eprintln!("after: {}", call_slot(0));

        // reset() = restore_all + disable in one call
        hook.reset();
        eprintln!("after reset: {}", call_slot(0));
    }
}
