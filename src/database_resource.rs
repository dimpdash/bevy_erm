use std::{
    any::type_name,
    sync::{Arc, RwLock},
};

use bevy_ecs::prelude::*;
use bevy_mod_index::index::Index;
use futures::executor::block_on;
use generational_arena::Arena;
use sqlx::Transaction;

use crate::*;

// Initially was going to have this trait to allow for implementing for different sql databases
// but the type system became too complex (for me)
// Now just use the sqlx::Sqlite database type
// Left the indirection in case I want to change it later

pub trait DatabaseResource: Resource + Default {
    type DatabaseConnection<'a>;

    fn get_connection(&self) -> Arc<DatabaseHandle>;
    // A way to get a unique key for the database
    fn get_key(&self) -> DatabaseEntityId;
    fn start_new_transaction(&self) -> RequestId;
    fn commit_transaction(&self, request: RequestId);
}

#[derive(Debug)]
pub struct DatabaseHandle {
    pub pool: RwLock<sqlx::SqlitePool>,

    // Require the option so that we can remove the transaction from the read write lock
    // when committing
    pub tr: RwLock<Arena<RwLock<Option<Transaction<'static, sqlx::Sqlite>>>>>,
    min_key: RwLock<i64>,
}

#[derive(Resource, Debug)]
pub struct AnyDatabaseResource {
    db: Arc<DatabaseHandle>,
}

impl Default for AnyDatabaseResource {
    fn default() -> Self {
        let pool = RwLock::new(block_on(sqlx::SqlitePool::connect("sqlite::memory:")).unwrap());
        let tr = RwLock::new(Arena::new());
        let db = Arc::new(DatabaseHandle {
            pool,
            tr,
            min_key: RwLock::new(0),
        });
        AnyDatabaseResource { db }
    }
}

unsafe impl Sync for DatabaseHandle {}
unsafe impl Send for DatabaseHandle {}

unsafe impl Sync for AnyDatabaseResource {}

#[macro_export]
macro_rules! get_transaction {
    ($name:ident, $request:expr, $db:expr) => {
        let database_handle = $db.get_connection();
        let arena = database_handle.tr.read().unwrap();
        let mut tr_lock = arena.get($request.0).unwrap().write().unwrap();
        let $name = tr_lock.as_mut().unwrap();
    };
}

impl DatabaseResource for AnyDatabaseResource {
    type DatabaseConnection<'a> = &'a mut sqlx::SqliteConnection;

    fn get_connection(&self) -> Arc<DatabaseHandle> {
        self.db.clone()
    }

    // Rather than actually querying the database for key just hold on to the last key we had to issue
    // This is a bit of a hack but it's fine for now. As POC and only considering one machine
    // It is progressing into the negatives so that instantiating any objects with positive keys will not conflict
    fn get_key(&self) -> DatabaseEntityId {
        let mut min_key = self.db.min_key.write().unwrap();
        *min_key -= 1;
        DatabaseEntityId(*min_key)
    }

    fn start_new_transaction(&self) -> RequestId {
        let mut transactions = self.db.tr.write().unwrap();
        let request = transactions.insert(RwLock::new(Some(
            block_on(self.db.pool.write().unwrap().begin()).unwrap(),
        )));
        RequestId(request)
    }

    fn commit_transaction(&self, request: RequestId) {
        let database_handle = self.get_connection();
        let mut arena = database_handle.tr.write().unwrap();
        let tr_lock = arena.remove(request.0).unwrap();
        let mut tr_lock_guard = tr_lock.write().unwrap();
        let tr = tr_lock_guard.take().unwrap();
        block_on(tr.commit()).unwrap();
    }
}

pub fn flush_component_to_db<T: ComponentMapper>(
    mut flush_events: EventReader<FlushEvent>,
    query: Query<(&DatabaseEntity, Option<&<T as ComponentMapper>::Component>)>,
    mut index: Index<RequestIdIndex>,
    db_query: DatabaseQuery<&T>,
) where
    <T as ComponentMapper>::Component: bevy_ecs::component::Component,
{
    println!("flushing component to db {}", type_name::<T>());
    for flush_event in flush_events.read() {
        for entity in index.lookup(&flush_event.request) {
            println!("flushing entity: {:?}", entity);
            if let (db_entity, Some(comp)) = query.get(entity).unwrap() {
                println!("db_entity: {:?}", db_entity);
                db_query
                    .update_or_insert_component(db_entity, comp)
                    .unwrap();
            }
        }
    }
}
