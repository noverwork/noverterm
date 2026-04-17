fn main() {
    eprintln!(
        "The migrator crate now uses Diesel CLI workflows. Use `diesel migration ...` in packages/migrator or `cargo make db:*` tasks."
    );
}
