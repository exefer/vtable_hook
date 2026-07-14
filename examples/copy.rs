#[derive(Debug)]
#[repr(C)]
pub struct CppClass {
    pub vtable: *const CppClassVTable,
}

impl Default for CppClass {
    fn default() -> Self {
        static VTABLE: CppClassVTable = CppClassVTable {
            foo: foo_bar_original,
            bar: foo_bar_original,
        };

        Self { vtable: &VTABLE }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct CppClassVTable {
    pub foo: unsafe extern "system" fn(thisptr: *const CppClass) -> std::os::raw::c_int,
    pub bar: unsafe extern "system" fn(thisptr: *const CppClass) -> std::os::raw::c_int,
}

unsafe extern "system" fn foo_bar_original(_thisptr: *const CppClass) -> std::os::raw::c_int {
    0
}

unsafe extern "system" fn bar_hooked(_thisptr: *const CppClass) -> std::os::raw::c_int {
    1
}

fn main() {
    unsafe {
        let mut victim_cpp_class = CppClass::default();
        let unaffected_cpp_class = CppClass::default();

        {
            let vtable_size =
                std::mem::size_of::<CppClassVTable>() / std::mem::size_of::<usize>();
            let mut hook = vtable_hook::hook::copy::Hook::new(
                &mut victim_cpp_class,
                None,
                Some(vtable_size),
            );
            eprintln!("Hook: {hook:#?}");

            eprintln!("-- Hook is disabled -- ");
            eprintln!(
                "victim_cpp_class's hook is_enabled {}",
                hook.is_enabled()
            );
            {
                let victim = &hook.item;
                eprintln!(
                    "victim_cpp_class bar() result = {}",
                    (victim.vtable.read().bar)(*victim),
                );
            }
            eprintln!(
                "unaffected_cpp_class bar() result = {}",
                (unaffected_cpp_class.vtable.read().bar)(&unaffected_cpp_class as *const _),
            );

            hook.replace_method(1, bar_hooked as vtable_hook::Method);
            hook.enable();

            eprintln!("-- Hook is enabled -- ");
            eprintln!(
                "victim_cpp_class's hook is_enabled {}",
                hook.is_enabled()
            );
            {
                let victim = &hook.item;
                eprintln!(
                    "victim_cpp_class bar() result = {}",
                    (victim.vtable.read().bar)(*victim),
                );
            }
            eprintln!(
                "unaffected_cpp_class bar() result = {}",
                (unaffected_cpp_class.vtable.read().bar)(&unaffected_cpp_class as *const _),
            );
        }

        eprintln!("-- Hook is disabled (drop) -- ");
        eprintln!(
            "victim_cpp_class bar() result = {}",
            (victim_cpp_class.vtable.read().bar)(&victim_cpp_class as *const _),
        );
        eprintln!(
            "unaffected_cpp_class bar() result = {}",
            (unaffected_cpp_class.vtable.read().bar)(&unaffected_cpp_class as *const _),
        );
    }
}
