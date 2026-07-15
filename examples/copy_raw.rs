use std::ffi::c_void;

use vtable_hook::{RawHook, RawVTable, VTable};

type VirtualFn = unsafe extern "system" fn(thisptr: *mut CppClass) -> i32;

#[derive(Debug)]
#[repr(C)]
struct CppClass {
    vtable: *const VirtualFn,
}

#[derive(Debug)]
#[repr(C)]
struct CppClassVTable {
    foo: VirtualFn,
}

static VTABLE: CppClassVTable = CppClassVTable { foo: foo_original };

unsafe extern "system" fn foo_original(_: *mut CppClass) -> i32 {
    0
}
unsafe extern "system" fn foo_hooked(_: *mut CppClass) -> i32 {
    1
}

fn main() {
    unsafe {
        let mut victim = CppClass {
            vtable: &VTABLE.foo,
        };
        let unaffected = CppClass {
            vtable: &VTABLE.foo,
        };

        let vtable = VTable::new_with_size(&VTABLE as *const _ as *const *const c_void, 1);
        let vptr_field: *mut *const c_void = &raw mut victim.vtable as *mut *const c_void;
        let mut hook = RawHook::new(vptr_field as *mut RawVTable, Some(vtable));
        eprintln!("hook: {hook:#?}");

        let call = |c: &CppClass| -> i32 {
            let f = std::mem::transmute::<*const c_void, VirtualFn>(*c.vtable as *const c_void);
            f(c as *const _ as *mut _)
        };

        eprintln!(
            "disabled: victim={} unaffected={}",
            call(&victim),
            call(&unaffected)
        );

        hook.hook(0, foo_hooked as *const c_void);

        eprintln!(
            "enabled: victim={} unaffected={}",
            call(&victim),
            call(&unaffected)
        );

        hook.reset();
        eprintln!(
            "reset: victim={} unaffected={}",
            call(&victim),
            call(&unaffected)
        );
    }
}
