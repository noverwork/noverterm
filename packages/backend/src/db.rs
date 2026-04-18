pub use crate::bootstrap::db::{init_pool, DbPool};

#[cfg(test)]
mod tests {
    use super::DbPool;

    #[test]
    fn db_pool_type_is_exposed_from_top_level_boundary() {
        let type_name = std::any::type_name::<DbPool>();

        assert!(type_name.contains("r2d2::Pool"));
    }
}
