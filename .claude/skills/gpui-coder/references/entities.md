# Entities, Ownership, and Signals

GPUI is retained state with declarative rendering. An `Entity<T>` survives across frames; the
element tree returned by `render` is a fresh description of one frame.

## Create, read, update

```rust
let draft = cx.new(|_| Draft { text: String::new() });   // from App or any Context<T>
let empty = draft.read(cx).text.is_empty();              // borrow ends before the next update
draft.update(cx, |draft, cx| {                           // inner cx is Context<Draft>
    draft.text.push_str("Hello");
    cx.notify();
});
```

- `cx.new` from a `Context<Parent>` gives its closure a **new** `Context<Child>`.
- A strong handle's `update` returns the closure's value; a `WeakEntity`'s returns `Result`.
- Finish a `read` before calling `update`; clone the small piece you need later.
- Never hold a `read` reference across an `await`.
- `cx.entity()` = strong handle to self. `cx.weak_entity()` = weak handle to self.

## Where state lives

| State | Owner |
|-------|-------|
| Saved data, sessions, current route | `Store` (`ui/store.rs`), reached with `store(cx)` |
| One page's transient state (search text, errors, loading) | That page's view |
| Text field / select behavior | `Entity<InputState>` / `Entity<SelectState<_>>` on the view |
| Hover, scroll offset, tiny element-local state | GPUI keyed element state (via `ElementId`) |
| App-wide service handle | A `Global` (`Backend`, `Globals`) |

Put each state in the narrowest owner that keeps it correct. One entity holding the whole app's
unrelated state is a failure mode; so is a second copy of a value that drifts from its owner.

Do not turn every visual fragment into an entity. Entities cost lifecycle and coordination; use
them where retained identity matters.

## Parent and child

```rust
struct Parent { child: Entity<Child> }          // strong: parent owns child
struct Child  { parent: WeakEntity<Parent> }    // weak: no cycle

// in a Child handler
let Some(parent) = this.parent.upgrade() else { return };
parent.update(cx, |parent, cx| { parent.clicks += 1; cx.notify(); });
```

A strong back-reference closes a cycle and neither entity is ever released. A missing parent is a
normal outcome, not an error.

Render a child entity by handing its handle to the tree: `.child(self.child.clone())`.
`AnyView` (`entity.into()`) erases the type, as `views::build` does for pages.

## observe vs subscribe

```rust
impl EventEmitter<StoreEvent> for Store {}

// in the subscriber's `new`
let _subscriptions = vec![
    cx.observe(&store, |_, _, cx| cx.notify()),                     // "it changed"
    cx.subscribe(&store, |this, _, event: &StoreEvent, cx| { ... }), // "this happened"
];
```

| | Fires on | Use when |
|---|---|---|
| `cx.observe(&entity, \|this, entity, cx\|)` | `cx.notify()` on the source | Only "state changed" matters |
| `cx.subscribe(&entity, \|this, entity, &event, cx\|)` | `cx.emit(event)` on the source | Meaning and payload matter |
| `cx.observe_in` / `cx.subscribe_in` | same | The callback also needs `&mut Window` |
| `cx.observe_global::<G>(...)` | the global changed | Following a `Global` |

- `notify` does not emit; `emit` does not notify. Do both deliberately when both are needed.
- Callbacks run in the effect cycle after the current update's borrow ends, not inside `update`.
- A method reference works as a callback: `cx.subscribe(&terminal, Self::on_terminal_event)`.

## Subscription lifetime

```rust
struct View { _subscriptions: Vec<Subscription> }
```

| What you do | Result |
|---|---|
| Store on the subscriber | Lives exactly as long as the view. Default. |
| Leave in a local `let` | Unsubscribed at the end of the function. Callback never runs. |
| `.detach()` | Stays until the **source** entity dies. Accumulates on long-lived sources. |

Never register a subscription from `render`: every render adds a duplicate.

## Globals

```rust
pub struct Backend(pub AppHandle);
impl Global for Backend {}

cx.set_global(Backend(app));                   // once, in ui::init
let app = cx.global::<Backend>().0.clone();
```

A global is for a genuinely app-wide service or registry. A model used by one feature is not
automatically a global — pass a cloned `Entity` handle instead.

## Lifecycle

An entity lives while one strong `Entity<T>` exists. Release runs in the effect cycle, not at the
`drop` statement. Cleanup should follow ownership: dropping a view drops its child entities, its
`_subscriptions`, and its stored `Task`s. `cx.on_release(...)` / `cx.observe_release(...)` exist for
integration code that must see state before it is dropped.

A view that stays alive unexpectedly: look for a strong entity cycle, a stored callback capturing a
strong handle, or a detached subscription/task outliving its owner. `cx.processor` captures a
strong handle, unlike `cx.listener`.

## Feedback loops

Inputs, selects, and filters have two paths: the owner sets a value, the user requests one. Do not
send an owner-supplied value back through the user callback during sync, and make callbacks safe
when they synchronously close or replace the component that invoked them.
