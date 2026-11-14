# Clippy & Correctness

## Project Lint Policy

No `[lints]` table and no `clippy.toml`: the default Clippy set, with every warning an error.

```bash
cargo make lint        # cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo make fmt-check   # cargo fmt --check
cargo make ci          # lint, fmt-check, tests
```

CI runs `cargo make lint`, `cargo make fmt-check`, and `cargo test --workspace --lib`. The
pre-commit hook runs the format check and Clippy on staged Rust changes.

Fix the cause of a lint. `#[allow(...)]` needs a comment saying why the lint is wrong here.

## Common Clippy Fixes

### Needless Borrow

```rust
// ❌ BAD
let s = &String::from("hello");

// ✅ GOOD
let s = String::from("hello");
```

### Manual Range Checks

```rust
// ❌ BAD
if x >= 0 && x <= 100 {
    use_value(x);
}

// ✅ GOOD
if (0..=100).contains(&x) {
    use_value(x);
}
```

### `map(|x| x.clone())`

```rust
// ❌ BAD
let items = values.iter().map(|x| x.clone()).collect::<Vec<_>>();

// ✅ GOOD
let items = values.iter().cloned().collect::<Vec<_>>();
```

## Formatting (Official)

Default rustfmt settings; there is no `rustfmt.toml`.
