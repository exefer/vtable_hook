#[derive(Debug)]
pub struct RawHook {
    struct_vtable_field_ptr: *mut crate::RawVTable,
    original_vtable: crate::VTable,
    our_vtable: Vec<crate::Method>,
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

        let our_vtable = unsafe { original_vtable.as_slice() }.to_vec();

        Self {
            struct_vtable_field_ptr,
            original_vtable,
            our_vtable,
        }
    }

    /// # Safety
    /// The vtable field pointer must still be valid (object not freed).
    pub unsafe fn is_enabled(&self) -> bool {
        let current_vtable_ptr = unsafe { self.struct_vtable_field_ptr.read_unaligned() };
        std::ptr::addr_eq(current_vtable_ptr, self.our_vtable.as_ptr())
    }

    /// # Safety
    /// The vtable field pointer must still be valid (object not freed).
    pub unsafe fn enable(&mut self) -> bool {
        if unsafe { self.is_enabled() } {
            return false;
        }

        unsafe {
            self.struct_vtable_field_ptr
                .replace(self.our_vtable.as_ptr());
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

    /// # Safety
    /// `index` must be within bounds of the original vtable.
    pub unsafe fn replace_method(&mut self, index: usize, our_method: crate::Method) -> Option<()> {
        let item = self.our_vtable.get_mut(index)?;
        *item = our_method;

        Some(())
    }

    /// # Safety
    /// `index` must be within bounds of the original vtable.
    pub unsafe fn restore_method(&mut self, index: usize) -> Option<()> {
        let item = self.our_vtable.get_mut(index)?;
        let original_method = unsafe { self.original_vtable.as_slice() }.get(index)?;
        *item = *original_method;

        Some(())
    }

    /// # Safety
    /// The original vtable must still be accessible.
    pub unsafe fn restore_all(&mut self) {
        let original_methods = unsafe { self.original_vtable.as_slice() };
        for (index, item) in self.our_vtable.iter_mut().enumerate() {
            let Some(original_method) = original_methods.get(index) else {
                continue;
            };

            *item = *original_method;
        }
    }
}
