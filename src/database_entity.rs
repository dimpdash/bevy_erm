use std::fmt::Display;

use bevy_ecs::component::Component;
use bevy_mod_index::prelude::*;
use bevy_ecs::prelude::*;

use crate::DatabaseEntityWithRequest;

pub trait RequestIdIndexInfo: IndexInfo<Value = RequestId> {}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Debug, sqlx::FromRow)]
pub struct Persisted(pub bool);

impl Default for Persisted {
    fn default() -> Self {
        Persisted(true)
    }
}

impl From<bool> for Persisted {
    fn from(b: bool) -> Self {
        Persisted(b)
    }
}

impl From<Persisted> for bool {
    fn from(val: Persisted) -> Self {
        val.0
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Debug, Default, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct DatabaseEntityId(pub i64);

impl Display for DatabaseEntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Component, Debug)]
pub struct DatabaseEntity {
    pub id: DatabaseEntityId,

    // Whether the entity has been persisted to the database ever
    // When creating an entity it will only be in memory and not have
    // been entered into the database yet, so will need to be inserted instead
    // updated
    pub persisted: Persisted,

    pub dirty: bool,

    // The request the database entity belongs to
    pub request: RequestId,
}

impl DatabaseEntityWithRequest for DatabaseEntity {
    fn request(&self) -> &RequestId {
        &self.request
    }
    fn id(&self) -> &DatabaseEntityId {
        &self.id
    }
}

pub struct DatabaseEntityIndex;
impl IndexInfo for DatabaseEntityIndex {
    type Component = DatabaseEntity;

    type Value = DatabaseEntityId;

    type Storage = NoStorage<Self>;

    fn value(c: &Self::Component) -> Self::Value {
        c.id
    }
}

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