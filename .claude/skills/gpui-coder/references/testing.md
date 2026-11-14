# Testing and Verification

Test at the lowest layer that proves the behavior.

## 1. Pure tests — the default

Keep logic out of views so it needs no GPUI: validation, parsing, ordering, formatting, color
choice. These are plain `#[test]`s next to the code.

```rust
fn validate(name: &str, port: &str) -> Result<i32, Errors> { ... }

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_an_out_of_range_port() {
        assert_eq!(validate("a", "70000").unwrap_err().port, Some("Port must be between 1 and 65535"));
    }
}
```

Existing examples: `validate` in `views/connection_form.rs`, `ui/stencil.rs`, `ui/i18n.rs`.
If a view method is hard to test, the logic inside it wants to be a free function.

```sh
cargo test -p desktop                 # everything
cargo test -p desktop stencil         # one module
```

## 2. Screenshots — looking at the result

`packages/desktop/tests/screenshots.rs` renders the real window offscreen with
`HeadlessAppContext`, seeds data, walks the routes, and writes PNGs.

```sh
cargo test -p desktop --test screenshots     # writes target/screenshots/*.png
```

Needs a GPU (Metal on macOS, WGPU on Linux). After a UI change, run it and **read the PNG** of the
page you touched. "It compiles" is not a UI check.

Adding a shot:

```rust
stage.store(|store, cx| store.navigate(Route::Keys, cx));
stage.settle(Duration::from_millis(400));   // lets backend work reach the views
stage.shoot("keys");
```

- `settle` sleeps in 50ms steps and calls `cx.run_until_parked()`; the backend runs on real Tokio,
  so the test executor's clock cannot fast-forward it.
- Drive state through the store or `cx.update_window(handle, |_, window, cx| ...)`, not by
  simulating pixels.
- `NOVERTERM_DATA_DIR` points the app at scratch data; `NOVERTERM_SKIP_UPDATE_CHECK` skips the
  request to GitHub.
- Call `window.close_all_dialogs(cx)` between shots that open dialogs.

## 3. `gpui_kit::test` — not available here

gpui-kit's `test-support` feature (`#[gpui_kit::test]`, `TestWindowExt`, `window.find("id")`,
`window.click`, `window.press`) pulls in `gpui-fast`, whose dependency pins clash with `russh`.
The repo therefore enables `test-support` on `gpui-pre` and `gpui-pre-platform` directly, in
`[dev-dependencies]`, and gets `TestAppContext` / `HeadlessAppContext` without the Kit helpers.

Do not add `features = ["test-support"]` to `gpui-kit`. If an interaction test is truly needed,
write it against `gpui-pre`'s `TestAppContext` and `VisualTestContext` and check the signatures in
`~/.cargo/registry/src/*/gpui-pre-0.3.12/src/app/test_app.rs` first.

## What to check for an interactive change

1. Pointer **and** keyboard activation reach the same code.
2. Disabled and busy states block a second submit.
3. Focus lands somewhere sensible after open, close, and delete.
4. Loading, empty, and error states each render (the screenshot seed covers non-empty pages).
5. Long text and the zh-TW translation do not break the layout.

## Before handing back

```sh
cargo fmt --all
cargo clippy -p desktop --all-targets
cargo test -p desktop
```

Report what was run and what was only read. Automated evidence and "I looked at the screenshot" are
different claims; say which one you have.
