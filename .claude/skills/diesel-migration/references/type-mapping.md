# SQL → Rust Type Mapping

Diesel maps PostgreSQL types to Rust types. The mapping depends on enabled Cargo features.

## Core types (always available)

| PostgreSQL | Diesel SQL Type | Rust Type |
|------------|----------------|-----------|
| `INTEGER` / `INT4` | `Int4` | `i32` |
| `BIGINT` / `INT8` | `Int8` | `i64` |
| `SMALLINT` / `INT2` | `Int2` | `i16` |
| `TEXT` / `VARCHAR` | `Text` | `String` |
| `BOOLEAN` | `Bool` | `bool` |
| `REAL` | `Float4` | `f32` |
| `DOUBLE PRECISION` | `Float8` | `f64` |
| `UUID` | `Uuid` | `uuid::Uuid` (requires `uuid` feature) |
| `JSON` / `JSONB` | `Json` | `serde_json::Value` (requires `serde_json` feature) |
| `BYTEA` | `Binary` | `Vec<u8>` |
| `NUMERIC` | `Numeric` | `bigdecimal::BigDecimal` (requires `numeric` feature) |

## Date/time types (requires `chrono` feature)

| PostgreSQL | Diesel SQL Type | Rust Type |
|------------|----------------|-----------|
| `TIMESTAMP` | `Timestamp` | `chrono::NaiveDateTime` |
| `TIMESTAMPTZ` | `Timestamptz` | `chrono::DateTime<Utc>` |
| `DATE` | `Date` | `chrono::NaiveDate` |
| `TIME` | `Time` | `chrono::NaiveTime` |

> **Critical**: `TIMESTAMP` maps to `NaiveDateTime`, `TIMESTAMPTZ` maps to `DateTime<Utc>`. Mismatching these causes compile errors.

## Nullable columns

Any column can be nullable. In `schema.rs` it appears as `Nullable<T>`. In Rust:
```rust
// schema.rs:  fingerprint -> Nullable<Text>,
// Rust model: pub fingerprint: Option<String>,
```

## Cargo.toml Feature Flags

The `orm` package must enable the correct diesel features:

```toml
[dependencies]
diesel = { version = "2.3", features = [
    "postgres",       # PostgreSQL backend
    "chrono",         # NaiveDateTime, DateTime<Utc> support
    "serde_json",     # JSON/JSONB support
    "uuid",           # UUID support
] }
chrono = { version = "0.4", features = ["serde"] }
```

**Missing a feature** causes compile errors like:
```
the trait bound `NaiveDateTime: AsExpression<Timestamp>` is not satisfied
```
