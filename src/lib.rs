pub mod hook;

pub type RawVTable = *const Method;
pub type Method = *const ();

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
        Self { begin: vtable, size }
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
    pub unsafe fn as_slice(&self) -> &[Method] {
        unsafe { std::slice::from_raw_parts(self.begin, self.size) }
    }
}
