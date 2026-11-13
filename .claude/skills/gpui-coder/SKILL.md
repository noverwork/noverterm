---
name: gpui-coder
description: Complete GPUI coding guide for the desktop app in this monorepo (packages/desktop, gpui-kit 0.9 over gpui-pre 0.3). Use when writing or changing any GPUI code - views, Render/RenderOnce components, entities, Context/Window/App usage, cx.notify, subscriptions, tasks, actions, key bindings, focus, layout, theme, or UI tests. Covers ownership rules, async patterns, and project conventions (Store, backend bridge, i18n, theme constants, shared page helpers). FORBIDS React/CSS/old-GPUI APIs translated by analogy (cx.new_view, View<T>, ViewContext, WindowContext, Model<T>) and direct `gpui::` imports.
---

# GPUI Coder - gpui-kit Guide

The desktop UI is GPUI through one facade crate: `gpui-kit`. `use gpui_kit::*;` is GPUI itself,
`gpui_kit::component::*` is the styled component library, `gpui_kit::base::*` is the unstyled
behavior layer.

GPUI changes fast and most examples online are stale. **Search the current source for a signature
before using it** — the nearest view in `packages/desktop/src/ui/`, then
`~/.cargo/registry/src/*/gpui-component-0.9.1`, `gpui-base-0.9.1`, `gpui-pre-0.3.12`. Do not invent a
method because the name seems plausible.

## Forbidden Patterns (Hard Blocks)

```rust
// ❌ NEVER — old GPUI names. They do not exist in gpui-pre 0.3.
cx.new_view(|cx| ...)            // ✅ cx.new(|cx| ...)
View<T>, Model<T>                // ✅ Entity<T>
ViewContext<T>, ModelContext<T>  // ✅ Context<T>
WindowContext                    // ✅ (window: &mut Window, cx: &mut App)

// ❌ NEVER — depend on or import `gpui` directly. The facade is the only entry point.
use gpui::*;                     // ✅ use gpui_kit::*;

// ❌ NEVER — create retained state in render. It is rebuilt every render and loses its state.
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let input = cx.new(|cx| InputState::new(window, cx)); // ✅ create in `new`, store on the view
    let focus = cx.focus_handle();                        // ✅ same
    cx.observe(&self.store, |_, _, cx| cx.notify());      // ✅ same, keep the Subscription
    cx.spawn(async move |this, cx| { ... }).detach();     // ✅ start from a handler or `new`
    cx.notify();                                          // ✅ never notify unconditionally in render
}

// ❌ NEVER — drop a handle you still need. Both stop working at the end of the statement.
cx.spawn(async move |this, cx| { ... });        // ✅ store the Task, or `.detach()` a one-shot
let _sub = cx.observe(&store, ...);             // ✅ keep it in `_subscriptions: Vec<Subscription>`

// ❌ NEVER — raw colors or radii in a view. Palette lives in `ui/theme.rs` only.
div().bg(rgb(0x101317)).rounded(px(2.))         // ✅ .bg(theme::PORT).rounded(theme::RADIUS)

// ❌ NEVER — untranslated user-facing text.
.label("Save")                                  // ✅ .label(t("Save")) + entry in i18n/zh-tw.json

// ❌ NEVER — a clickable div where a component exists (loses focus, keyboard, disabled, a11y).
div().id("save").on_click(...)                  // ✅ Button::new("save").ghost().on_click(...)
```

## Core Types Quick Reference

| Type | What it is | Rule |
|------|------------|------|
| `Entity<T>` | Strong handle to retained state | Owner stores it; clone the handle, not the data |
| `WeakEntity<T>` | Non-owning handle | Child → parent, async tasks, long-lived callbacks |
| `Context<T>` | `cx` for the current entity | `notify`, `emit`, `listener`, `spawn`, `observe`, `subscribe` |
| `App` | `cx` with no current entity | Globals, `cx.new`, reading/updating other entities |
| `Window` | Focus, actions, dialogs, geometry | Passed as `window`, always before `cx` |
| `Render` | Trait of an entity-backed view | `render(&mut self, &mut Window, &mut Context<Self>)` |
| `RenderOnce` | Trait of a value-like component | `render(self, &mut Window, &mut App)`, needs `#[derive(IntoElement)]` |
| `SharedString` | Cheap-to-clone UI text | All text kept across frames or moved into closures |
| `Subscription` | RAII handle of observe/subscribe | Dropped = unsubscribed |
| `Task<T>` | RAII handle of spawned work | Dropped = cancelled |
| `FocusHandle` | Keyboard focus identity | Created once, attached with `.track_focus(&handle)` |
| `ElementId` | Stable identity of an element | From a domain id, never from an index or a label |

Never store `&mut Window`, `&mut App`, or `&mut Context<_>`. Store handles.

## Essential Patterns

### A view — `Render` on an entity

```rust
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{h_flex, v_flex, Sizable as _};
use gpui_kit::*;

pub struct KeysView {
    store: Entity<Store>,
    search: Entity<InputState>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

impl KeysView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = store(cx);
        let search = cx.new(|cx| InputState::new(window, cx).placeholder(t("Search")));
        let _subscriptions = vec![
            cx.observe(&store, |_, _, cx| cx.notify()),
            cx.subscribe(&search, |_, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            }),
        ];
        Self { store, search, error: None, _subscriptions }
    }
}

impl Render for KeysView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        common::page(t("Keys"), self.render_actions(cx), self.render_list(cx))
    }
}
```

Keep `render` declarative: read state, derive values, compose. Split named regions into
`render_<region>(&self, cx)` helpers. Mutation, parsing, and requests go in named methods.

### Callbacks — `cx.listener`

Component handlers receive `(&Event, &mut Window, &mut App)`, not the view. `cx.listener` gives the
view back through a weak handle:

```rust
Button::new(("delete-key", index))
    .ghost()
    .label(t("Delete"))
    .on_click(cx.listener(move |this, _, window, cx| this.request_delete(&key, window, cx)))
```

Handlers are `'static`: move owned values (`SharedString`, a cloned `Entity`, an id) into them.

### Mutate, then notify once

```rust
fn loaded(&mut self, result: Result<Vec<Key>, String>, cx: &mut Context<Self>) {
    self.loading = false;
    match result {
        Ok(keys) => self.keys = keys,
        Err(error) => self.error = Some(error.into()),
    }
    cx.notify(); // changing a field alone does not redraw
}
```

`cx.notify()` = "my state changed, redraw and run observers". `cx.emit(event)` = typed fact for
subscribers. Neither implies the other.

### Value-like component — `RenderOnce`

Default for stateless reusable pieces. It has no `Context<Self>`: no `cx.listener`, no `cx.notify`,
no subscriptions. The owner passes values and handlers in.

```rust
#[derive(IntoElement)]
struct StatusPill {
    label: SharedString,
    live: bool,
}

impl RenderOnce for StatusPill {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .text_xs()
            .text_color(if self.live { theme::GREEN_TEXT } else { theme::STENCIL_MUTE })
            .child(self.label)
    }
}
```

In this repo most shared pieces are plain functions returning `Div` (`views/common.rs`); use a
`RenderOnce` struct when the piece needs builders.

### Controlled components

Pass the current value in, receive the requested change, update the owner, notify. No second copy
of the state.

```rust
Switch::new("use-key")
    .checked(self.use_key)
    .on_change(cx.listener(|this, checked: &bool, _, cx| {
        this.use_key = *checked;
        cx.notify();
    }))
```

Stateful components split in two: `Entity<InputState>` / `Entity<SelectState<_>>` lives on the
view, `Input::new(&self.name)` / `Select::new(&self.group)` is rebuilt every render.

### Async — backend work from a view

```rust
fn remove(&mut self, host: String, port: u16, cx: &mut Context<Self>) {
    let app = store(cx).read(cx).app();
    cx.spawn(async move |this, cx| {
        let removed = backend(async move {
            crate::trust::known_hosts_remove(host, port, app.state()).await
        })
        .await;
        let _ = this.update(cx, |this, cx| this.loaded(removed, cx));
    })
    .detach();
}
```

`this` is a `WeakEntity<Self>`; a failed `update` means the view closed, which is normal. Backend
futures use Tokio, so they **must** go through `backend(...)` — the UI executor cannot drive them.

## Critical Pitfalls

### 1. Stale UI — state changed without `cx.notify()`

Every mutation that affects rendering ends in exactly one `cx.notify()` on the owning entity.

### 2. Re-entrant entity access panics

Inside an entity's `render`, `update`, or `cx.listener`, do not `read`/`update` that same entity
through its handle. Use `self`/`this` and the inner `cx`. For work that must run after the current
update, use `cx.defer_in(window, |this, window, cx| ...)`.

```rust
self.store.update(cx, |store, cx| store.navigate(route, cx)); // ✅ inner cx
```

### 3. `spawn` has no window

`cx.spawn` gives `AsyncApp`. To focus, open a dialog, or read bounds after an `await`, use
`cx.spawn_in(window, async move |this, cx| ...)` with `this.update_in(cx, |this, window, cx| ...)`.

### 4. Strong cycles keep views alive

Parent owns child with `Entity<Child>`; child refers back with `WeakEntity<Parent>`. Async tasks
and stored callbacks capture weak handles (`cx.weak_entity()`). `cx.listener`, `cx.observe`, and
`cx.subscribe` are already weak; an extra `move` of a strong handle is not.

### 5. `Task::detach` ≠ `Subscription::detach`

`task.detach()`: one-shot work runs to completion. Fine. `subscription.detach()`: the callback stays
until the source entity dies. On a long-lived source such as `Store`, that accumulates callbacks —
store the `Subscription` instead.

### 6. Late async results overwrite new state

A request can finish after a newer one. Store the task in an `Option<Task<()>>` field (replacing it
cancels the old one) and/or compare a revision before applying.

### 7. Unstable `ElementId` resets state

Ids key hover, scroll, focus, and animation state. Build them from domain ids
(`SharedString::from(format!("remove-{}", host.id))` or `("remove", index)` only for fixed lists).
Never from a translated label, a random value, or the index of a reorderable list.

### 8. `h_flex` centers, `v_flex` stretches

A column inside a bare `h_flex()` takes its content height and gets centered, clipping its header.
Rows of columns need `.items_stretch()` (or `.h_full()` on the column).

### 9. Flex children do not shrink

A scrolling or truncating child of a flex container needs `.min_h_0()` / `.min_w_0()`, and a scroll
area needs an `.id(...)`: `div().id("list").flex_1().min_h_0().overflow_y_scroll()`.

### 10. Missing trait import

`.when(...)` / `.when_some(...)` need `gpui_kit::prelude::FluentBuilder as _`. `.small()`,
`.disabled(...)`, `.primary()` need `Sizable as _`, `Disableable as _`, `ButtonVariants as _`.
`window.open_alert_dialog` needs `component::WindowExt as _`.

## Project Conventions

1. **Always** import GPUI as `use gpui_kit::*;`; import component traits anonymously (`as _`)
2. **Always** keep subscriptions in `_subscriptions: Vec<Subscription>` on the view
3. **Always** wrap user-facing text in `t("English source")` / `tf("… {name}", &[("name", v)])` and add the `zh-tw.json` entry
4. **Always** take colors and radius from `crate::ui::theme`; a new color is a new constant there
5. **Prefer** `views/common.rs` helpers (`page`, `page_body`, `form_page`, `form_buttons`, `form_section`, `field`, `card`, `empty_state`, `error_banner`, `confirm_delete`, `new_select`) over a new layout
6. **Use** `store(cx)` for shared app state; mutate it with `store.update(cx, |store, cx| ...)`, follow it with `cx.observe(&store, |_, _, cx| cx.notify())`
7. **Use** `lucide("kebab-name")` for icons and rem helpers (`gap_3()`, `px_4()`, `text_sm()`, `size_10()`) over `px(...)`
8. **Signature order**: event/action first, then `window`, then `cx`; omit `window` when unused; the context is always named `cx`
9. **Avoid** unnecessary secondary labels and descriptions; let the layout do the explaining
10. **Represent** every async operation as explicit state (`loading`, `error: Option<SharedString>`, data) and show the error in the UI, not only in logs
11. **New page**: one module in `ui/views/`, a `Route` variant in `ui/store.rs`, an arm in `views::build`
12. **Filename**: `snake_case.rs`, named after the product concept (`known_hosts.rs`, not `list_view.rs`)

## Data Fetching Pattern

```rust
pub struct KnownHostsView {
    hosts: Vec<TrustedSshHost>,
    loading: bool,
    error: Option<SharedString>,
}

impl KnownHostsView {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let app = store(cx).read(cx).app();
        cx.spawn(async move |this, cx| {
            let loaded = backend(async move { crate::trust::known_hosts_get(app.state()).await }).await;
            let _ = this.update(cx, |this, cx| this.loaded(loaded, cx));
        })
        .detach();
        Self { hosts: Vec::new(), loading: true, error: None }
    }
}

impl Render for KnownHostsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = if self.loading {
            common::empty_state(t("Loading...")).into_any_element()
        } else if self.hosts.is_empty() {
            common::empty_state(t("No Known Hosts yet.")).into_any_element()
        } else {
            div()
                .grid()
                .grid_cols(3)
                .gap_3()
                .children(self.hosts.iter().map(|host| self.render_card(host, cx)))
                .into_any_element()
        };
        common::page(
            t("Known Hosts"),
            div(),
            common::page_body("known-hosts")
                .children(self.error.clone().map(|error| common::error_banner(error).mb_4()))
                .child(body),
        )
    }
}
```

Branches of different element types need `.into_any_element()`. Optional children go through
`.children(option.map(...))` or `.when_some(option, |el, value| ...)`.

## Form Pattern

```rust
pub struct KeyForm {
    name: Entity<InputState>,
    errors: Errors,              // one Option<&'static str> per field
    error: Option<SharedString>, // the save failure
    saving: bool,
}

fn save(&mut self, cx: &mut Context<Self>) {
    let name = self.name.read(cx).value().trim().to_string();
    self.errors = Errors { name: name.is_empty().then_some("Name is required") };
    if self.errors != Errors::default() || self.saving {
        cx.notify();
        return;
    }
    self.saving = true; // blocks a second submit
    cx.notify();
    // spawn → backend(...) → this.update: clear `saving`, set `error` or navigate away
}

// in render
common::form_page(
    t("New Key"),
    self.error.clone(),
    common::field(t("Name"), Input::new(&self.name), self.errors.name.map(t)),
    common::form_buttons(
        t("Save"),
        self.saving,
        cx.listener(|this, _, _, cx| this.cancel(cx)),
        cx.listener(|this, _, _, cx| this.save(cx)),
    ),
)
```

Keep validation a pure function returning `Errors` so it has a plain `#[test]`.

## Before Writing UI

Name these first; if one is unclear, read the nearest existing view:

1. the user's task and the object on screen
2. the state owner (which entity) and what is merely derived
3. the component to compose from (check `gpui_kit::component` before building from `div`s)
4. the keyboard path and where focus goes
5. loading, empty, and error states
6. the check that fails if it regresses (pure test, or a screenshot to look at)

## Detailed References

- **Entities** (ownership, weak handles, observe/subscribe, events, globals, lifecycle) → `references/entities.md`
- **Async** (spawn, spawn_in, background work, backend bridge, cancellation, stale results) → `references/async.md`
- **Actions, keys, focus** (actions!, KeyBinding, key_context, FocusHandle, dialogs) → `references/actions-focus.md`
- **Layout and styling** (flex, scrolling, theme, rem scale, ids, interface copy) → `references/layout-styling.md`
- **Testing** (pure tests, screenshot harness, what `test-support` can and cannot do here) → `references/testing.md`
- Upstream guides: <https://gpui-kit.com/docs/coding-guides>, full text at <https://gpui-kit.com/llms-full.txt>
