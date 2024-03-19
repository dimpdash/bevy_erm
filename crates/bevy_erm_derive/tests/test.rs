//test
#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use bevy_erm_core::ComponentMapper;
    use bevy_erm_derive::DBQueryDerive;
    use sqlx::prelude::FromRow;

    use bevy_erm_core::database_query::{CustomDatabaseQuery, DatabaseTransaction};

    pub trait Easy {
        fn easy(&self) -> i32;
    }

    #[derive(DBQueryDerive, FromRow)]
    #[table_name = "test"]
    #[allow(dead_code)]
    struct Test {
        id: i32,
        name: String,
        price: f32,
    }

    #[derive(DBQueryDerive, FromRow)]
    #[table_name = "test"]
    struct MarkerTest {}
}
