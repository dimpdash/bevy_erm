pub mod database_entity;
pub mod database_query;
pub mod database_resource;

pub use database_entity::*;
pub use database_resource::*;

use database_query::{DatabaseEntityWithRequest, DatabaseQuery as DatabaseQueryInternal};
pub type DatabaseQuery<'w, 's, DBQueryInfo> =
    DatabaseQueryInternal<'w, 's, DBQueryInfo, AnyDatabaseResource>;

pub use database_query::ComponentMapper;
