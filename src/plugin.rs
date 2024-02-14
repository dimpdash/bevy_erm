
use std::{cell::RefCell, sync::RwLock};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use crate::*;
use futures::executor::block_on;

pub struct EntityRelationMapperPlugin{
    // use RwLock so can have interio mutability
    // use option so can take ownership of it
    pub config: RwLock<Option<EntityRelationalMapperConfig>>, 
}

pub struct EntityRelationalMapperConfig
{
    // pub still_events_to_read: F,
    pub flush_to_db_schedule: Schedule,
}

impl EntityRelationMapperPlugin {
    pub fn create_runner(&self, mut h : EntityRelationalMapperConfig) -> impl FnOnce(App) + 'static + Send {
        move |mut app: App | {
            //TODO
            // while (h.still_events_to_read)(&mut app.world) {
            //     println!("===========================");
            //     app.update();
            // }
            // println!("===========================");
        
            // (h.flush_to_db_schedule).run(&mut app.world);
        
            // let mut commit_schedule = Schedule::default();
            // commit_schedule.add_systems(commit_transaction);
            // commit_schedule.run(&mut app.world);
        
            // println!("All Events Processed");
        
            // let mut new_transation_schedule = Schedule::default();
            // new_transation_schedule.add_systems(start_new_transaction);
            // new_transation_schedule.run(&mut app.world);
        }
    }
}

impl Plugin for EntityRelationMapperPlugin {
    fn build(&self, app: &mut App) {

        let holder = self.config.write().unwrap().take().unwrap();

        app.set_runner(self.create_runner(holder))
            .init_resource::<AnyDatabaseResource>();
    }
}


fn commit_transaction(db: Res<AnyDatabaseResource>) {
    block_on(async {
        let db_handle = db.get_connection();
        let tr_option = &mut (*db_handle).write().unwrap().tr;
        let tr = tr_option.take().unwrap();
        tr.commit().await.unwrap();
    });
}

fn start_new_transaction(db: Res<AnyDatabaseResource>) {
    block_on(async {
        let db_handle = db.get_connection();
        let new_transaction = {
            let pool = &(*db_handle).write().unwrap().pool;
            pool.begin().await.unwrap()
        };
        let tr_option = &mut (*db_handle).write().unwrap().tr;
        *tr_option = Some(new_transaction);
    });
}


