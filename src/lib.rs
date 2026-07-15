//! Hook C++ virtual tables at runtime by cloning and replacing the vtable pointer.
//!
//! Two hook types:
//! - `Hook<T>` - lifetime-safe, auto-disables on drop.
//! - `RawHook` - raw pointer based, caller manages safety.

pub mod hook;

use std::ffi::c_void;

/// Pointer to the first entry of a vtable array.
///
/// Dereferencing yields a function pointer (`*const c_void`).
/// The vtable may be null-terminated (zero entry marks the end)
/// or have a known fixed size.
pub type RawVTable = *const *const c_void;

/// A vtable snapshot: a pointer to its first entry and the entry count.
///
/// Created from a `RawVTable` either by counting null-terminated entries
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
