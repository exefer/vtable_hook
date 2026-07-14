use std::ffi::c_void;

/// Low-level vtable hook handle. The caller is responsible for ensuring
/// the target object outlives this handle.
///
/// On construction, the original vtable is cloned. Methods can be replaced
/// in the clone, and `enable` swaps the object's vptr to point at it.
/// `disable` restores the original vptr. The clone is freed on drop but
/// the vptr is NOT restored - the caller must `disable` first or let
/// `Hook<T>` handle it.
#[derive(Debug)]
pub struct RawHook {
    struct_vtable_field_ptr: *mut crate::RawVTable,
    original_vtable: crate::VTable,
    patched_vtable: Vec<crate::Method>,
}

impl RawHook {
    /// # Safety
    /// `struct_vtable_field_ptr` must point to valid, writable memory
    /// for the lifetime of this hook. If `original_vtable` is `None`,
    /// the pointer must dereference to a valid null-terminated vtable.
    pub unsafe fn new(
        struct_vtable_field_ptr: *mut crate::RawVTable,
        original_vtable: Option<crate::VTable>,
    ) -> Self {
        let original_vtable = match original_vtable {
            Some(some) => some,
            None => unsafe { crate::VTable::new(struct_vtable_field_ptr.read_unaligned()) },
        };

        let patched_vtable = unsafe { original_vtable.as_slice() }.to_vec();

        Self {
            struct_vtable_field_ptr,
            original_vtable,
            patched_vtable,
        }
    }

    /// # Safety
    /// The vtable field pointer must still be valid (object not freed).
    pub unsafe fn is_enabled(&self) -> bool {
        let current_vtable_ptr = unsafe { self.struct_vtable_field_ptr.read_unaligned() };
        std::ptr::addr_eq(current_vtable_ptr, self.patched_vtable.as_ptr())
    }

    /// # Safety
    /// The vtable field pointer must still be valid (object not freed).
    pub unsafe fn enable(&mut self) -> bool {
        if unsafe { self.is_enabled() } {
            return false;
        }

        unsafe {
            self.struct_vtable_field_ptr
                .replace(self.patched_vtable.as_ptr());
        }

        true
    }

    /// # Safety
    /// The vtable field pointer must still be valid (object not freed).
    pub unsafe fn disable(&mut self) -> bool {
        if !unsafe { self.is_enabled() } {
            return false;
        }

        unsafe {
            self.struct_vtable_field_ptr
                .replace(self.original_vtable.begin);
        }

        true
    }

    /// Returns the original function pointer at `index` as a raw pointer.
    ///
    /// # Panics
    /// If `index` is out of bounds.
    pub fn original(&self, index: usize) -> *const c_void {
        let methods = unsafe { self.original_vtable.as_slice() };
        methods[index] as *const c_void
    }

    /// Returns the original function pointer at `index` cast to type `F`.
    ///
    /// # Safety
    /// `F` must be a pointer-sized type (e.g., a function pointer).
    pub unsafe fn original_fn<F>(&self, index: usize) -> F {
        let ptr = self.original(index);
        unsafe { std::mem::transmute_copy::<*const c_void, F>(&ptr) }
    }

    /// One-shot: replace method at `index` and enable the hook.
    ///
    /// # Safety
    /// `index` must be within bounds of the vtable.
    /// `hook_fn` must match the calling convention of the original method.
    pub unsafe fn hook(&mut self, index: usize, hook_fn: crate::Method) {
        unsafe { self.replace_method(index, hook_fn) };
        unsafe { self.enable() };
    }

    /// Restore the original method at `index`.
    ///
    /// # Safety
    /// `index` must be within bounds of the original vtable.
    pub unsafe fn unhook(&mut self, index: usize) {
        unsafe { self.restore_method(index) };
    }

    /// Restore all original methods and disable the hook.
    ///
    /// # Safety
    /// The original vtable must still be accessible.
    pub unsafe fn reset(&mut self) {
        unsafe { self.restore_all() };
        unsafe { self.disable() };
    }

    /// # Safety
    /// `index` must be within bounds of the original vtable.
    pub unsafe fn replace_method(&mut self, index: usize, hook_fn: crate::Method) {
        self.patched_vtable[index] = hook_fn;
    }

    /// # Safety
    /// `index` must be within bounds of the original vtable.
    pub unsafe fn restore_method(&mut self, index: usize) {
        self.patched_vtable[index] = unsafe { self.original_vtable.as_slice() }[index];
    }

    /// # Safety
    /// The original vtable must still be accessible.
    pub unsafe fn restore_all(&mut self) {
        let original_methods = unsafe { self.original_vtable.as_slice() };
        for (index, item) in self.patched_vtable.iter_mut().enumerate() {
            let Some(original_method) = original_methods.get(index) else {
                continue;
            };

            *item = *original_method;
        }
    }
}
