# Actions, Key Bindings, and Focus

## Action or callback

| Need | Use |
|---|---|
| A command reachable from a key, a menu, and a button | An `Action` |
| A pointer-only interaction on one element | `.on_click(cx.listener(...))` |
| A typed fact for another entity | `cx.emit` + `cx.subscribe` (see `entities.md`) |

Model one command once. A toolbar button, a menu item, and a key binding dispatch the same action
or call the same owner method; do not copy the mutation into each.

## Define

```rust
actions!(noverterm, [OpenSettings, ShowTerminal, CloseTab, NextTab, PreviousTab]);

/// Jumps to the tab at this index.
#[derive(Clone, PartialEq, Debug, Action)]
#[action(namespace = noverterm, no_json)]
pub struct SelectTab(pub usize);
```

Use the `actions!` re-exported by `gpui_kit` (in scope through `use gpui_kit::*;`). GPUI's own
macro expands to `gpui::Action`, which does not resolve through the facade.

## Bind

```rust
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-,", OpenSettings, None),            // global
        KeyBinding::new("tab", SendTab, Some("Terminal")),       // only inside that key context
        KeyBinding::new("ctrl-w", CloseTab, Some("!Terminal && !Input")),
    ]);
}
```

- Bind once at startup, from `ui::init` (`shell::bind_keys`, `terminal::bind_keys`).
- `cmd` on macOS, `ctrl` elsewhere — see the `modifier` helper in `shell::bind_keys`.
- The third argument is a key-context predicate. `None` applies everywhere.
- Bind keys **before** `cx.set_menus`: menus bake shortcuts in when they are built.

## Handle

```rust
div()
    .track_focus(&self.focus)
    .key_context("Terminal")
    .on_action(cx.listener(|this, _: &SendTab, _, cx| this.send_tab(false, cx)))
    .on_action(cx.listener(|this, action: &SelectTab, _, cx| this.select_tab(action.0, cx)))
```

`key_context` and its `on_action` handlers go on the **same** focused region. Dispatch walks from
the focused element up to the root: an action with no handler on that path does nothing. Window-
level commands are handled on the shell's root element.

Stop propagation (`cx.stop_propagation()`) only when a nested interaction must keep its parent from
handling the same event. Blanket stops break menus, selection, and window-level commands.

## Focus

```rust
struct Pane { focus: FocusHandle }

// in new
focus: cx.focus_handle(),

// in render
div().track_focus(&self.focus)

// move focus
self.focus.focus(window, cx);

// let others ask for it
impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() }
}
terminal.focus_handle(cx).focus(window, cx);
```

- Create the handle once on the owner. A handle made in `render` is new every frame.
- A tracked handle is a Tab stop only when built as `cx.focus_handle().tab_stop(true)`.
- Never request focus unconditionally from `render`. Do it in the handler that opens the thing.
- Read `self.focus.is_focused(window)` in render for a simple focus style.

### Reacting to focus changes

```rust
let _subscriptions = vec![
    cx.on_focus_in(&focus, window, |this, _, cx| { this.active = true; cx.notify(); }),
    cx.on_focus_out(&focus, window, |this, _, _, cx| { this.active = false; cx.notify(); }),
];
```

Register in `new`, keep the `Subscription`s. `on_focus`/`on_blur` are exact; `_in`/`_out` include
tracked descendants.

### When the focused element disappears

If a view stops rendering the focused element (a row is deleted, a list becomes an empty state),
the window still holds the old handle but it is no longer in the dispatch tree — the view's
shortcuts go dead. Either move focus in the same update that removes the element, or register
`cx.on_focus_lost(window, |this, window, cx| ...)` once on the root view and focus
`window.focus_lost_restore_target(cx)` or a fallback handle.

## Dialogs, notifications, overlays

Use the window's layers; they trap and restore focus for you.

```rust
use gpui_kit::component::WindowExt as _;

window.open_dialog(cx, move |dialog, _, cx| dialog.title(t("Update")).child(...));
window.close_dialog(cx);
window.open_alert_dialog(cx, |alert, _, _| alert.title(...).confirm().on_ok(|_, _, _| true));
window.push_notification(Notification::error(message), cx);
```

- Deleting something: `common::confirm_delete(...)`. Do not hand-roll a confirmation.
- No confirmation dialog for reversible, low-risk actions.
- A dialog body is built by a closure that may run again; keep it side-effect-free and move owned
  values in (`Rc` for a non-`Clone` callback, as `confirm_delete` does).
- Do not build a menu, select, or popover from `div`s; the component owns keyboard navigation,
  dismissal, and accessibility.
- One `Root` per window, created by `gpui_kit::open_window`. Never a second root per page.

## Raw key events

`.on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| ...))` is for input that is not a
command (the terminal's keystrokes). `event.keystroke.modifiers.secondary(cx)` is Cmd on macOS and
Ctrl elsewhere. Anything a user would call a shortcut is an action with a binding.

## Missing shortcut checklist

1. Is the action bound (`bind_keys` runs at startup)?
2. Does the binding's context predicate match the focused element's `key_context` chain?
3. Is the `on_action` handler on an element that is an ancestor of the focused one?
4. Is anything focused at all, and is it still rendered?
5. Did a nearer handler call `stop_propagation`?
