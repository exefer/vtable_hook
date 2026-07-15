//! Hook C++ virtual tables at runtime by cloning and replacing the vtable pointer.
//!
//! Two hook types:
//! - [`Hook<T>`] — lifetime-safe, auto-disables on drop.
//! - [`RawHook`] — raw pointer based, caller manages safety.

use std::ffi::c_void;

/// Pointer to the first entry of a vtable array.
///
/// Dereferencing yields a function pointer (`*const c_void`).
/// The vtable may be null-terminated (zero entry marks the end)
/// or have a known fixed size.
pub type RawVTable = *const *const c_void;

/// A vtable snapshot: a pointer to its first entry and the entry count.
///
/// Created from a [`RawVTable`] either by counting null-terminated entries
/// (`new`) or by supplying an explicit size (`new_with_size`).
#[derive(Debug, Clone)]
pub struct VTable {
    pub begin: RawVTable,
    pub size: usize,
}

impl VTable {
    /// # Safety
    /// `vtable` must point to a valid vtable terminated by a null entry.
    pub unsafe fn new(vtable: RawVTable) -> Self {
        unsafe { Self::new_with_size(vtable, Self::count_methods_raw(vtable)) }
    }

    /// # Safety
    /// `vtable` must point to at least `size` readable entries.
    pub unsafe fn new_with_size(vtable: RawVTable, size: usize) -> Self {
        Self {
            begin: vtable,
            size,
        }
    }

    /// # Safety
    /// `vtable` must point to a vtable with a null sentinel entry.
    pub unsafe fn count_methods_raw(mut vtable: RawVTable) -> usize {
        let mut size = 0;
        while !unsafe { std::ptr::read(vtable) }.is_null() {
            unsafe { vtable = vtable.add(1) };
            size += 1;
        }
        size
    }

    /// # Safety
    /// The vtable must be valid for `self.size` entries.
    pub unsafe fn as_slice(&self) -> &[*const c_void] {
        unsafe { std::slice::from_raw_parts(self.begin, self.size) }
    }
}

/// Low-level vtable hook handle. The caller is responsible for ensuring
/// the target object outlives this handle.
///
/// On construction, the original vtable is cloned. Methods can be replaced
/// in the clone, and `enable` swaps the object's vptr to point at it.
/// `disable` restores the original vptr. The clone is freed on drop but
/// the vptr is NOT restored — the caller must `disable` first or let
/// [`Hook<T>`] handle it.
#[derive(Debug)]
pub struct RawHook {
    struct_vtable_field_ptr: *mut RawVTable,
    original_vtable: VTable,
    patched_vtable: Vec<*const c_void>,
}

impl RawHook {
    /// # Safety
    /// `struct_vtable_field_ptr` must point to valid, writable memory
    /// for the lifetime of this hook. If `original_vtable` is `None`,
    /// the pointer must dereference to a valid null-terminated vtable.
    pub unsafe fn new(
        struct_vtable_field_ptr: *mut RawVTable,
        original_vtable: Option<VTable>,
    ) -> Self {
        let original_vtable = original_vtable
            .unwrap_or_else(|| unsafe { VTable::new(struct_vtable_field_ptr.read_unaligned()) });

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
                .replace(self.patched_vtable.as_ptr())
        };

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
                .replace(self.original_vtable.begin)
        };

        true
    }

    /// Returns the original function pointer at `index` as a raw pointer.
    ///
    /// # Panics
    /// If `index` is out of bounds.
    pub fn original(&self, index: usize) -> *const c_void {
        let methods = unsafe { self.original_vtable.as_slice() };
        methods[index]
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
    pub unsafe fn hook(&mut self, index: usize, hook_fn: *const c_void) {
        self.replace_method(index, hook_fn);
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

    /// # Panics
    /// If `index` is out of bounds.
    pub fn replace_method(&mut self, index: usize, hook_fn: *const c_void) {
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

/// A vtable hook tied to a specific object's lifetime.
///
/// Automatically disables the hook (restores the original vptr) when
/// `Hook` is dropped. Methods delegate to the inner [`RawHook`].
#[derive(Debug)]
pub struct Hook<'a, T> {
    pub item: &'a mut T,
    raw: RawHook,
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
        let item_ptr = &raw mut *item as *mut usize;

        let vtable_offset = vtable_offset.unwrap_or(0);
        let struct_vtable_field_ptr = unsafe { item_ptr.add(vtable_offset) } as *mut RawVTable;
        let vtable = unsafe { struct_vtable_field_ptr.read_unaligned() };
        let vtable_size =
            methods_count.unwrap_or_else(|| unsafe { VTable::count_methods_raw(vtable) });
        let original_vtable = unsafe { VTable::new_with_size(vtable, vtable_size) };

        let raw = unsafe { RawHook::new(struct_vtable_field_ptr, Some(original_vtable)) };
        Self { item, raw }
    }

    pub fn is_enabled(&self) -> bool {
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
    pub unsafe fn hook(&mut self, index: usize, hook_fn: *const c_void) {
        unsafe { self.raw.hook(index, hook_fn) }
    }

    /// # Panics
    /// If `index` is out of bounds.
    pub fn unhook(&mut self, index: usize) {
        unsafe { self.raw.unhook(index) }
    }

    /// # Safety
    /// The original vtable must still be accessible.
    pub unsafe fn reset(&mut self) {
        unsafe { self.raw.reset() }
    }

    /// # Panics
    /// If `index` is out of bounds.
    pub fn replace_method(&mut self, index: usize, hook_fn: *const c_void) {
        self.raw.replace_method(index, hook_fn)
    }

    /// # Panics
    /// If `index` is out of bounds.
    pub fn restore_method(&mut self, index: usize) {
        unsafe { self.raw.restore_method(index) }
    }

    pub fn restore_all(&mut self) {
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
