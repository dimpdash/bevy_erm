pub mod database_entity;
pub mod database_query;
pub mod database_resource;
pub mod plugin;

use std::fmt::Display;

use bevy_ecs_macros::Event;
use bevy_mod_index::{index::IndexInfo, storage::NoStorage};
pub use database_entity::*;
pub use database_query::*;
pub use database_resource::*;
pub use plugin::*;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Debug, Hash)]
pub struct RequestId(pub generational_arena::Index);

impl Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (i, gen) = self.0.into_raw_parts();
        write!(f, "{}v{}", i, gen)
    }
}

#[derive(Event)]
pub struct FlushEvent {
    pub request: RequestId,
}

impl DatabaseEntityWithRequest for (DatabaseEntityId, RequestId) {
    fn request(&self) -> &RequestId {
        &self.1
    }

    fn id(&self) -> &DatabaseEntityId {
        &self.0
    }
}

impl DatabaseEntityWithRequest for (RequestId, DatabaseEntityId) {
    fn request(&self) -> &RequestId {
        &self.0
    }

    fn id(&self) -> &DatabaseEntityId {
        &self.1
    }
}

pub struct RequestIdIndex;
impl IndexInfo for RequestIdIndex {
    type Component = DatabaseEntity;

    type Value = RequestId;

    type Storage = NoStorage<Self>;

    fn value(c: &Self::Component) -> Self::Value {
        c.request
    }
}
