use std::ffi::c_void;

use vtable_hook::Hook;

type VirtualFn = unsafe extern "system" fn(thisptr: *const CppClass) -> i32;

#[derive(Debug)]
#[repr(C)]
struct CppClass {
    vtable: *const CppClassVTable,
}

#[derive(Debug)]
#[repr(C)]
struct CppClassVTable {
    foo: VirtualFn,
    bar: VirtualFn,
}

static VTABLE: CppClassVTable = CppClassVTable {
    foo: foo_bar_original,
    bar: foo_bar_original,
};

unsafe extern "system" fn foo_bar_original(_: *const CppClass) -> i32 {
    0
}
unsafe extern "system" fn bar_hooked(_: *const CppClass) -> i32 {
    1
}

fn call(c: &CppClass) -> i32 {
    unsafe { (c.vtable.read().bar)(c as *const _) }
}

fn main() {
    let mut victim = CppClass { vtable: &VTABLE };
    let unaffected = CppClass { vtable: &VTABLE };

    let vtable_size = size_of::<CppClassVTable>() / size_of::<usize>();
    let mut hook = unsafe { Hook::new(&mut victim, None, Some(vtable_size)) };
    eprintln!("hook: {hook:#?}");

    eprintln!(
        "disabled: victim={} unaffected={}",
        call(hook.item),
        call(&unaffected)
    );

    unsafe { hook.hook(1, bar_hooked as *const c_void) };

    eprintln!(
        "enabled: victim={} unaffected={}",
        call(hook.item),
        call(&unaffected)
    );
}
