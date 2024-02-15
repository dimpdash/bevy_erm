pub mod database_entity;
pub mod database_query;
pub mod database_resource;
pub mod plugin;

use bevy_ecs_macros::Event;
pub use database_entity::*;
pub use database_query::*;
pub use database_resource::*;
pub use plugin::*;


#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Debug, Default)]    
pub struct RequestId(pub i64);

#[derive(Event)]
pub struct FlushEvent{
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