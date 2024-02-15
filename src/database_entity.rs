use bevy_ecs::{component::Component, prelude::*};
use bevy_mod_index::prelude::*;
use bevy_utils::petgraph::visit::Data;

use crate::{DatabaseEntityWithRequest, RequestId};

pub struct DatabaseEntityIndex;
impl IndexInfo for DatabaseEntityIndex {
    type Component = DatabaseEntity;

    type Value = i64;

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

pub type DatabaseEntityId = i64;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Component, Debug, Default, sqlx::FromRow)]
pub struct DatabaseEntity {
    pub id: DatabaseEntityId,

    // Whether the entity has been persisted to the database ever
    // When creating an entity it will only be in memory and not have
    // been entered into the database yet, so will need to be inserted instead
    // updated
    #[sqlx(skip)]
    pub persisted: Persisted,

    #[sqlx(skip)]
    pub dirty: bool,

    #[sqlx(skip)]
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