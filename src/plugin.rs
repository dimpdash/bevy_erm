use std::sync::RwLock;

use crate::*;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

pub struct EntityRelationMapperPlugin;

impl Plugin for EntityRelationMapperPlugin{
    fn build(&self, app: &mut App) {
        app.add_event::<FlushEvent>()
            .init_resource::<AnyDatabaseResource>();
    }
}

