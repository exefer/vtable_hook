# vtable_hook

Hook C++ virtual tables at runtime by cloning and swapping the vptr.

## Install

```toml
[dependencies]
vtable_hook = "0.3"
```

## API

Two hook types:

- `Hook<T>` - borrows a typed object, auto-disables on drop.
- `RawHook` - operates on a raw vptr pointer, caller manages safety.

Core methods (both types):

- `hook(index, fn)` - replace a method and enable in one call.
- `get_original(index)` - retrieve the original method at a slot.
- `enable()` / `disable()` - swap vptr in and out.
- `reset()` - restore all methods and disable.

## Example

See [examples/audit_minimal.rs](examples/audit_minimal.rs).
