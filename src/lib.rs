pub mod database_entity;
pub mod database_query;
pub mod database_resource;
pub mod plugin;

use std::fmt::Display;

use bevy_ecs_macros::Event;
use bevy_mod_index::{index::IndexInfo, storage::NoStorage};
pub use database_entity::*;
pub use database_resource::*;
pub use plugin::*;

use database_query::{DatabaseQuery as DatabaseQueryInternal, DatabaseEntityWithRequest};
pub type DatabaseQuery<'w, 's, DBQueryInfo> = DatabaseQueryInternal<'w, 's, DBQueryInfo, AnyDatabaseResource>;

pub use database_query::ComponentMapper;