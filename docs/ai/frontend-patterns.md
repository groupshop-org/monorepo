# Frontend Patterns

Reactive state management and async patterns for `packages/frontend`.

## State: Mutable vs Arc<Mutex>

Use `Mutable<T>` **only** when the value drives reactive UI via `.signal()` / `.signal_cloned()` / `.signal_ref()`. Examples: visibility toggles, loading indicators, error banners that appear/disappear, list contents that re-render.

Use `Arc<Mutex<T>>` (or `Arc<RwLock<T>>`) for **non-reactive state** — values read or written imperatively but never observed through a signal. Examples: form field values read on submit, abort handles, accumulated results. This avoids the overhead of the signal notification machinery when no signal is ever polled.

Quick test: grep for the variable name + `.signal`. If there are zero signal observations, it should be `Arc<Mutex<T>>`.

## Async: AsyncLoader

`dominator_helpers::AsyncLoader` wraps `futures::future::abortable` and `spawn_local` into a convenient handle that:

- Cancels any in-flight future when `.load()` is called again (swap semantics).
- Cancels the future when the `AsyncLoader` is dropped.
- Provides `.is_loading()` as a `Signal<Item = bool>` for reactive UI.

Prefer `AsyncLoader` over manually combining `AbortHandle` + `spawn_local`. It lives in `frontend-shared/src/util/async_loader.rs` and is re-exported via the prelude.

```rust
let loader = AsyncLoader::new();

// Start an async task (cancels any previous one automatically)
loader.load(clone!(state => async move {
    // ... async work ...
}));

// Observe loading state in the DOM
.class_signal("loading", loader.is_loading())

// Cancel explicitly if needed
loader.cancel();
```

When the `AsyncLoader` is stored inside a struct/closure that is dropped (e.g. modal DOM removed), the in-flight future is automatically cancelled — no manual cleanup needed.

## Choosing Between Patterns

| Need | Use |
|---|---|
| Value drives UI reactively | `Mutable<T>` |
| Plain state, no signal observation | `Arc<Mutex<T>>` or `Arc<RwLock<T>>` |
| One-shot async (fire and forget) | `spawn_local(async { ... })` |
| Cancellable async with loading signal | `AsyncLoader` |
| Multiple concurrent reads, rare writes | `Arc<RwLock<T>>` |
