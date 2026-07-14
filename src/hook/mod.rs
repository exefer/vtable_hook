/// VTable hook strategies.
///
/// Each submodule implements a different hooking technique.
/// `copy` clones the vtable and swaps the vptr.
pub mod copy;
