pub mod database_entity;
pub mod database_query;
pub mod database_resource;

pub use database_entity::*;
pub use database_resource::*;

use database_query::{DatabaseEntityWithRequest, DatabaseQuery as DatabaseQueryInternal};
pub type DatabaseQuery<'w, 's, DBQueryInfo> =
    DatabaseQueryInternal<'w, 's, DBQueryInfo, AnyDatabaseResource>;

pub use database_query::ComponentMapper;

use generational_arena::Index as GenIndex;
use sqlx::FromRow;
use sqlx::Row;

pub struct DataseBaseEntityAndComponent<C> {
    pub entity: DatabaseEntity,
    pub component: C,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for DatabaseEntity {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(DatabaseEntity {
            id: row.try_get("id")?,
            persisted: Persisted(true),
            dirty: false,
            //Change when instantiated
            request: RequestId(GenIndex::from_raw_parts(0, 0)),
        })
    }
}

impl<'a, C: FromRow<'a, sqlx::sqlite::SqliteRow>> FromRow<'a, sqlx::sqlite::SqliteRow>
    for DataseBaseEntityAndComponent<C>
{
    fn from_row(row: &'a sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(DataseBaseEntityAndComponent {
            entity: DatabaseEntity::from_row(row)?,
            component: C::from_row(row)?,
        })
    }
}
