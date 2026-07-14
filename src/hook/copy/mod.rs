use std::ffi::c_void;

/// Copy-based vtable hooking: clone the target vtable, patch the clone,
/// then swap the object's vptr to point at the patched copy.
///
/// The raw submodule provides the core `RawHook`. This module wraps it in
/// a lifetime-safe `Hook<T>` that auto-disables on drop.
pub mod raw;

#[derive(Debug)]
/// A vtable hook tied to a specific object's lifetime.
///
/// Automatically disables the hook (restores the original vptr) when
/// `Hook` is dropped. Methods delegate to the inner `RawHook`.
pub struct Hook<'a, T> {
    pub item: &'a mut T,
    raw: raw::RawHook,
}

impl<'a, T> Hook<'a, T> {
    /// # Safety
    /// `item` must be a live polymorphic C++ object with a valid vtable
    /// pointer at `vtable_offset`. If `methods_count` is `None`, the
    /// vtable must be null-terminated.
    pub unsafe fn new(
        item: &'a mut T,
        vtable_offset: Option<usize>,
        methods_count: Option<usize>,
    ) -> Self {
        let item_ptr = item as *mut _ as *mut usize;

        let vtable_offset = vtable_offset.unwrap_or(0);
        let struct_vtable_field_ptr =
            unsafe { item_ptr.add(vtable_offset) } as *mut crate::RawVTable;
        let vtable = unsafe { struct_vtable_field_ptr.read_unaligned() };
        let vtable_size = match methods_count {
            Some(n) => n,
            None => unsafe { crate::VTable::count_methods_raw(vtable) },
        };
        let original_vtable = unsafe { crate::VTable::new_with_size(vtable, vtable_size) };

        let raw = unsafe { raw::RawHook::new(struct_vtable_field_ptr, Some(original_vtable)) };
        Self { item, raw }
    }

    /// # Safety
    /// The underlying object must still be alive.
    pub unsafe fn is_enabled(&self) -> bool {
        unsafe { self.raw.is_enabled() }
    }

    /// # Safety
    /// The underlying object must still be alive.
    pub unsafe fn enable(&mut self) -> bool {
        unsafe { self.raw.enable() }
    }

    /// # Safety
    /// The underlying object must still be alive.
    pub unsafe fn disable(&mut self) -> bool {
        unsafe { self.raw.disable() }
    }

    /// # Panics
    /// If `index` is out of bounds.
    pub fn original(&self, index: usize) -> *const c_void {
        self.raw.original(index)
    }

    /// # Safety
    /// `F` must be a function pointer type (same size as `*const ()`).
    pub unsafe fn original_fn<F>(&self, index: usize) -> F {
        unsafe { self.raw.original_fn(index) }
    }

    /// # Safety
    /// `index` must be within the vtable bounds.
    /// `hook_fn` must match the calling convention of the original method.
    pub unsafe fn hook(&mut self, index: usize, hook_fn: crate::Method) {
        unsafe { self.raw.hook(index, hook_fn) }
    }

    /// # Safety
    /// `index` must be within the vtable bounds.
    pub unsafe fn unhook(&mut self, index: usize) {
        unsafe { self.raw.unhook(index) }
    }

    /// # Safety
    /// The original vtable must still be accessible.
    pub unsafe fn reset(&mut self) {
        unsafe { self.raw.reset() }
    }

    /// # Safety
    /// `index` must be within the vtable bounds.
    pub unsafe fn replace_method(&mut self, index: usize, hook_fn: crate::Method) {
        unsafe { self.raw.replace_method(index, hook_fn) }
    }

    /// # Safety
    /// `index` must be within the vtable bounds.
    pub unsafe fn restore_method(&mut self, index: usize) {
        unsafe { self.raw.restore_method(index) }
    }

    /// # Safety
    /// The original vtable must still be accessible.
    pub unsafe fn restore_all(&mut self) {
        unsafe { self.raw.restore_all() }
    }
}

impl<'a, T> Drop for Hook<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.raw.disable();
        }
    }
}
