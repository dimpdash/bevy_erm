use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

use bevy_ecs::{component::ComponentId, prelude::*};
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

pub trait ComponentMapperMapper {
    fn update_or_insert_component(
        db_entity: DatabaseEntity,
        entity: Entity,
        component_type_id: TypeId,
        type_id: ComponentId,
        request: RequestId,
        world: &mut World,
    ) -> Result<(), ()>;
}

// #[macro_export]
// macro_rules! impl_flush_component_to_db {
//     ($($name:ident)+) => {
//         use bevy_mod_index::index::Index;
//         use bevy_ecs::prelude::*;

//         pub fn flush_component_to_db(
//             flush_events: EventReader<FlushEvent>,
//             index: Index<RequestIdIndex>,
//             query: Query<(&DatabaseEntity, $(Option<&<$name as ComponentMapper>::Component>,)+)>,
//             db_query: DatabaseQuery<($(&$name, )+)>
//         ) {
//             for flush_event in flush_events.read() {
//                 for entity in index.lookup(&flush_event.request) {
//                     let (db_entity, $(lower!($name), )+) = query.get(entity).unwrap();

//                     $(
//                         if let Some(comp) = lower!($name) {
//                             db_query
//                                 .update_or_insert_component(db_entity, comp)
//                                 .unwrap();
//                         }
//                     )+
//                 }

//                 println!("Committing transaction");
//                 db.commit_transaction(flush_event.request);
//             }
//         }
//     };
// }

pub fn flush_component_to_db<'w1, 'w2, 's, DBQ: DBQueryInfo>(
    mut flush_events: EventReader<FlushEvent>,
    mut index: Index<RequestIdIndex>,
    db_query: DatabaseQuery<DBQ>,
) where
    'w1: 'w2,
    's: 'w2,
{
    for flush_event in flush_events.read() {
        for entity in index.lookup(&flush_event.request) {
            db_query.update_or_insert_component(entity).unwrap();
        }

        db_query.commit(flush_event.request).unwrap();
    }
}
