use std::{
    any::TypeId,
    sync::{Arc, RwLock},
};

use bevy_ecs::{component::ComponentId, prelude::*};
use bevy_mod_index::index::Index;
use futures::lock::Mutex;

use futures::executor::block_on;
use generational_arena::Arena;
use sqlx::Transaction;

use crate::database_entity::*;
use crate::database_query::*;

// Initially was going to have this trait to allow for implementing for different sql databases
// but the type system became too complex (for me)
// Now just use the sqlx::Sqlite database type
// Left the indirection in case I want to change it later

pub trait DatabaseResource: Resource + Default {
    type Transaction: Send;

    // A way to get a unique key for the database
    fn get_key(&self) -> DatabaseEntityId;
    fn start_new_transaction(&self) -> RequestId;
    fn try_start_new_transaction(&self) -> Option<RequestId>;
    fn get_transaction(&self, request: RequestId) -> Self::Transaction;

    fn commit_transaction(&self, request: RequestId);
}

#[derive(Debug)]
pub struct DatabaseHandle {
    pub pool: RwLock<sqlx::SqlitePool>,

    // Require the option so that we can remove the transaction from the read write lock
    // when committing
    pub tr: RwLock<Arena<Arc<Mutex<A>>>>,
    min_key: RwLock<i64>,
}

#[derive(Resource, Debug)]
pub struct SqlxSqliteDatabaseResource {
    db: DatabaseHandle,
}

impl Default for SqlxSqliteDatabaseResource {
    fn default() -> Self {
        let pool = RwLock::new(
            block_on(
                sqlx::pool::PoolOptions::new()
                    .min_connections(3)
                    .connect("sqlite::memory:"),
            )
            .unwrap(),
        );
        let tr = RwLock::new(Arena::new());
        let db = DatabaseHandle {
            pool,
            tr,
            min_key: RwLock::new(0),
        };
        SqlxSqliteDatabaseResource { db }
    }
}

unsafe impl Sync for DatabaseHandle {}
unsafe impl Send for DatabaseHandle {}

unsafe impl Sync for SqlxSqliteDatabaseResource {}

#[derive(Debug)]
pub struct A {
    pub a: Option<Transaction<'static, sqlx::Sqlite>>,
}

unsafe impl Send for A {}
unsafe impl Sync for A {}

impl DatabaseResource for SqlxSqliteDatabaseResource {
    type Transaction = Arc<Mutex<A>>;

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
        let transaction = block_on(self.db.pool.write().unwrap().begin()).unwrap();
        let request = transactions.insert(Arc::new(Mutex::new(A {
            a: Some(transaction),
        })));
        RequestId(request)
    }

    fn try_start_new_transaction(&self) -> Option<RequestId> {
        let mut transactions = self.db.tr.write().unwrap();
        let transaction = block_on(self.db.pool.write().unwrap().try_begin()).unwrap()?;
        let request = transactions.insert(Arc::new(Mutex::new(A {
            a: Some(transaction),
        })));
        Some(RequestId(request))
    }

    fn commit_transaction(&self, request: RequestId) {
        block_on(async {
            let database_handle = &self.db;
            let mut arena = database_handle.tr.write().unwrap();
            let tr_lock = arena.remove(request.0).unwrap();
            let tr_lock_guard = tr_lock.lock();
            let tr = tr_lock_guard.await.a.take().unwrap();
            tr.commit().await.unwrap();
        })
    }

    fn get_transaction(&self, request: RequestId) -> Self::Transaction {
        let database_handle = &self.db;
        let arena = database_handle.tr.read().unwrap();
        arena.get(request.0).unwrap().clone()
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

pub fn flush_component_to_db<
    'w1,
    'w2,
    's,
    DBQ: DBQueryInfo<DbResource>,
    DbResource: DatabaseResource,
>(
    mut flush_events: EventReader<FlushEvent>,
    mut index: Index<RequestIdIndex>,
    db_query: DatabaseQuery<DBQ, DbResource>,
) where
    'w1: 'w2,
    's: 'w2,
{
    for flush_event in flush_events.read() {
        for entity in index.lookup(&flush_event.request) {
            block_on(db_query.update_or_insert_component(entity)).unwrap();
        }

        db_query.commit(flush_event.request).unwrap();
    }
}
