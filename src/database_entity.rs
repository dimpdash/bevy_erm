use std::fmt::Display;

use bevy_ecs::{component::Component, prelude::*};
use bevy_mod_index::prelude::*;
use bevy_utils::petgraph::visit::Data;

use crate::{DatabaseEntityWithRequest, RequestId};

pub struct DatabaseEntityIndex;
impl IndexInfo for DatabaseEntityIndex {
    type Component = DatabaseEntity;

    type Value = DatabaseEntityId;

    type Storage = NoStorage<Self>;

    fn value(c: &Self::Component) -> Self::Value {
        c.id
    }
}

pub fn add_event<T>(world: &mut World)
where
    T: Event,
{
    if !world.contains_resource::<Events<T>>() {
        world.init_resource::<Events<T>>();
    }
}

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
        write!(f, "DatabaseEntityId({})", self.0)
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Component, Debug, Default)]
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