# Clippy & Correctness

## Project Lint Policy

```toml
[lints.clippy]
correctness = "deny"
pedantic = "deny"

missing_errors_doc = "allow"
missing_panics_doc = "allow"
module_name_repetitions = "allow"
must_use_candidate = "allow"
```

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

```bash
cargo fmt --workspace
cargo fmt --workspace -- --check
```

Use default rustfmt settings unless there is a very strong project-wide reason not to.

## Clippy (Project)

```bash
cargo clippy --workspace -- -D warnings
```

## cargo-deny (Project)

```bash
cargo deny check
```
