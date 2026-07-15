# vtable_hook

Hook C++ virtual tables at runtime by cloning and swapping the vptr.

## API

Two hook types:

- `Hook<T>` - borrows a typed object, auto-disables on drop.
- `RawHook` - operates on a raw vptr pointer, caller manages safety.

Core methods (both types):

- `hook(index, fn)` - replace a method and enable in one call.
- `original(index)` - retrieve the original method at a slot.
- `enable()` / `disable()` - swap vptr in and out.
- `reset()` - restore all methods and disable.

## Examples

See the [examples](examples/) folder.
