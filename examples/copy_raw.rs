#[derive(Debug)]
#[repr(C)]
pub struct CppClass {
    pub vtable: *const CppClassVTable,
}

impl Default for CppClass {
    fn default() -> Self {
        static VTABLE: CppClassVTable = CppClassVTable { foo: foo_original };

        Self { vtable: &VTABLE }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CppClassVTable {
    pub foo: unsafe extern "system" fn(thisptr: *const CppClass) -> std::os::raw::c_int,
}

unsafe extern "system" fn foo_original(_thisptr: *const CppClass) -> std::os::raw::c_int {
    0
}

unsafe extern "system" fn foo_hooked(_thisptr: *const CppClass) -> std::os::raw::c_int {
    1
}

fn main() {
    unsafe {
        let mut victim_cpp_class = CppClass::default();
        let unaffected_cpp_class = CppClass::default();

        let original_vtable = vtable_hook::VTable::new_with_size(
            victim_cpp_class.vtable as vtable_hook::RawVTable,
            1,
        );
        let mut raw_hook = vtable_hook::hook::copy::raw::RawHook::new(
            &mut victim_cpp_class.vtable as *mut _ as *mut vtable_hook::RawVTable,
            Some(original_vtable),
        );
        eprintln!("Raw hook: {raw_hook:#?}");

        eprintln!("-- Hook is disabled -- ");
        eprintln!(
            "victim_cpp_class's raw_hook is_enabled {}",
            raw_hook.is_enabled(),
        );
        eprintln!(
            "victim_cpp_class foo() result = {}",
            (victim_cpp_class.vtable.read().foo)(&victim_cpp_class as *const _),
        );
        eprintln!(
            "unaffected_cpp_class foo() result = {}",
            (unaffected_cpp_class.vtable.read().foo)(&unaffected_cpp_class as *const _),
        );

        raw_hook.replace_method(0, foo_hooked as vtable_hook::Method);
        raw_hook.enable();

        eprintln!("-- Hook is enabled -- ");
        eprintln!(
            "victim_cpp_class's raw_hook is_enabled {}",
            raw_hook.is_enabled(),
        );
        eprintln!(
            "victim_cpp_class foo() result = {}",
            (victim_cpp_class.vtable.read().foo)(&victim_cpp_class as *const _),
        );
        eprintln!(
            "unaffected_cpp_class foo() result = {}",
            (unaffected_cpp_class.vtable.read().foo)(&unaffected_cpp_class as *const _),
        );

        raw_hook.disable();

        eprintln!("-- Hook is disabled -- ");
        eprintln!(
            "victim_cpp_class's raw_hook is_enabled {}",
            raw_hook.is_enabled(),
        );
        eprintln!(
            "victim_cpp_class foo() result = {}",
            (victim_cpp_class.vtable.read().foo)(&victim_cpp_class as *const _),
        );
        eprintln!(
            "unaffected_cpp_class foo() result = {}",
            (unaffected_cpp_class.vtable.read().foo)(&unaffected_cpp_class as *const _),
        );
    }
}
