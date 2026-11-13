# Async Work

## Two executors

The UI runs on GPUI's executor. The backend (SSH, SFTP, SQLite, HTTP) runs on Tokio
(`crate::app::runtime()`). A backend future polled on the UI executor has no Tokio reactor and
fails. `ui/bridge.rs` crosses over:

```rust
use crate::ui::bridge::backend;

let result = backend(async move { crate::store::keys::key_list(app.state()).await }).await;
```

`backend(future)` spawns on Tokio and returns a future the UI task can await. Everything moved in
must be `Send + 'static`: clone the `AppHandle` and owned inputs before spawning.

## Start work from an owner

```rust
fn search(&mut self, cx: &mut Context<Self>) {
    let query = self.query.read(cx).value().to_string();  // extract before spawning
    self._search = Some(cx.spawn(async move |this, cx| {
        let results = backend(async move { search_index(query).await }).await;
        _ = this.update(cx, |this, cx| {
            this.results = results;
            cx.notify();
        });
    }));
}
```

- Start from a handler, a named method, or `new`. Never from `render`.
- `this` is `WeakEntity<Self>`, `cx` is `AsyncApp`. No borrow of `self` or `cx` crosses an `await`.
- A failed `this.update` means the view closed. Use `_ =`, `.ok()`, or `?` on purpose.

## Keep, await, or detach

| Choice | When |
|---|---|
| Store `Task<()>` / `Option<Task<()>>` on the view | Search, refresh, debounce, streams, anything recurring. Dropping the view cancels it; assigning a new task cancels the previous one. |
| `.await` it from another task | The next step needs its result. |
| `.detach()` | A one-shot that should finish even if the view closes (a save, a delete, the initial load). |

`cx.spawn(...);` as a bare statement drops the handle and may cancel the work before it starts.

Cancellation stops further polling. It does not undo I/O already sent or interrupt a blocking call.

## Needing the window after an await

```rust
fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    cx.spawn_in(window, async move |this, cx| {
        let saved = backend(async move { ... }).await;
        _ = this.update_in(cx, |this, window, cx| {
            this.saving = false;
            this.name.focus_handle(cx).focus(window, cx);
            cx.notify();
        });
    })
    .detach();
}
```

`spawn` → `AsyncApp` → `this.update(cx, |this, cx|)`.
`spawn_in` → `AsyncWindowContext` → `this.update_in(cx, |this, window, cx|)`.
From plain `App::spawn` there is no entity: use `cx.update(|cx| ...)`.

## Stale results

```rust
self.revision = self.revision.wrapping_add(1);
let revision = self.revision;
self._load = Some(cx.spawn(async move |this, cx| {
    let loaded = backend(async move { ... }).await;
    _ = this.update(cx, |this, cx| {
        if this.revision != revision {
            return; // an older result must not replace newer state
        }
        this.apply(loaded, cx);
    });
}));
```

Replacing the stored task is usually enough. Add the revision check when a result can already be
queued when the replacement happens.

## CPU-heavy work

`cx.background_spawn(async move { parse(source) }).await` inside a `cx.spawn` task. The worker gets
owned `Send` input and has no `App`, `Window`, or `Context`; apply its result back in
`this.update`. Never block the UI executor: no `std::thread::sleep`, no blocking I/O, no
`while !ready {}` polling.

## Explicit operation state

```rust
loading: bool,
error: Option<SharedString>,
saving: bool,
```

1. Set `loading`/`saving`, clear `error`, `cx.notify()`, then spawn.
2. On completion: reject stale, store data or a readable error, clear the flag, notify **once**.
3. Disable or `.loading(busy)` the submit button while busy — no duplicate destructive submits.
4. Keep previous data visible during a refresh when possible.

A failure only logged, or dropped by `.detach()`, leaves the user looking at a spinner forever.

## Backend events

`Store` listens to the backend's named events and re-emits them as `StoreEvent::Backend(name,
json)`. A view that follows one subscribes to the store:

```rust
cx.subscribe(&store, |this, _, event: &StoreEvent, cx| {
    if let StoreEvent::Backend("link_status", payload) = event {
        this.on_status(payload, cx);
    }
})
```

Add a new backend event name to `BACKEND_EVENTS` in `ui/store.rs`.

## Deferring

`cx.defer_in(window, |this, window, cx| ...)` runs after the current update releases its borrow —
for focus restoration after the tree changes, or for updating an entity that is currently borrowed.
It is not a durable job queue: a closed window drops it.
