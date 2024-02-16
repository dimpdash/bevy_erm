use std::sync::RwLock;

use crate::*;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

pub struct EntityRelationMapperPlugin {
    flush_schedule: RwLock<Option<Schedule>>,
}

impl EntityRelationMapperPlugin {
    pub fn new() -> Self {
        let mut flush_schedule = Schedule::new(PostUpdate);
        flush_schedule.add_systems(commit_transaction);
        EntityRelationMapperPlugin {
            flush_schedule: RwLock::new(Some(flush_schedule)),
        }
    }
    pub fn add_flush_system<M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        self.flush_schedule
            .write()
            .unwrap()
            .as_mut()
            .unwrap()
            .add_systems(systems.before(commit_transaction));

        self
    }
}

impl Plugin for EntityRelationMapperPlugin {
    fn build(&self, app: &mut App) {
        let mut flush_schedule = self.flush_schedule.write().unwrap().take().unwrap();

        flush_schedule.initialize(&mut app.world).unwrap();

        app.add_event::<FlushEvent>()
            .add_schedule(flush_schedule)
            .init_resource::<AnyDatabaseResource>();
    }
}

fn commit_transaction(db: Res<AnyDatabaseResource>, mut flush_events: EventReader<FlushEvent>) {
    for flush_event in flush_events.read() {
        db.commit_transaction(flush_event.request);
    }
}
