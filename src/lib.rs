pub mod database_entity;
pub mod database_query;
pub mod database_resource;
pub mod plugin;

use bevy_ecs_macros::Event;
pub use database_entity::*;
pub use database_query::*;
pub use database_resource::*;
pub use plugin::*;


pub type RequestId = i64;

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