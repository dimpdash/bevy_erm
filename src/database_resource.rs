use std::sync::{Arc, RwLock};

use bevy_ecs::prelude::*;
use futures::executor::block_on;
use sqlx::Transaction;

use crate::*;
use crate::database_entity::DatabaseEntity;

// use self::database_query::DBQueryInfo;


// Initially was going to have this trait to allow for implementing for different sql databases
// but the type system became too complex (for me)
// Now just use the sqlx::Sqlite database type
// Left the indirection in case I want to change it later

pub trait DatabaseResource: Resource + Default {
    type DatabaseConnection<'a>;
    
    fn get_connection(&self) -> Arc<RwLock<DatabaseHandle>>;
    // A way to get a unique key for the database
    fn get_key(&self) -> i64;


}

#[derive(Debug)]
pub struct DatabaseHandle {
    pub pool: sqlx::SqlitePool,
    // Currently on handle one transaction at a time
    // IMPROVEMNET: Use a vec of transactions to allow for multiple transactions at once
    pub tr: Option<Transaction<'static, sqlx::Sqlite>>,
    min_key: i64,
}

#[derive(Resource, Debug)]
pub struct AnyDatabaseResource {
    db: Arc<RwLock<DatabaseHandle>>,
}

impl Default for AnyDatabaseResource {
    fn default() -> Self {
        let pool = block_on(sqlx::SqlitePool::connect("sqlite::memory:")).unwrap();
        let tr = Some(block_on(pool.begin()).unwrap());
        let db = Arc::new(RwLock::new(DatabaseHandle { pool, tr, min_key: 0}));
        AnyDatabaseResource { db }
    }
}

unsafe impl Sync for DatabaseHandle {}
unsafe impl Send for DatabaseHandle {}

unsafe impl Sync for AnyDatabaseResource {}

impl DatabaseResource for AnyDatabaseResource {
    type DatabaseConnection<'a> = &'a mut sqlx::SqliteConnection;

    fn get_connection(&self) -> Arc<RwLock<DatabaseHandle>> {
        self.db.clone()
    }

    // Rather than actually querying the database for key just hold on to the last key we had to issue
    // This is a bit of a hack but it's fine for now. As POC and only considering one machine
    // It is progressing into the negatives so that instantiating any objects with positive keys will not conflict
    fn get_key(&self) -> i64 {
        let mut db = self.db.write().unwrap();
        db.min_key -= 1;
        db.min_key

    }
}

pub fn flush_component_to_db<T: ComponentMapper>(
    query: Query<(&DatabaseEntity, &<T as ComponentMapper>::Component)>,
    db_query: DatabaseQuery<&T>,
)
    where <T as ComponentMapper>::Component: bevy_ecs::component::Component
{
    for (db_entity, component) in query.iter() {
        db_query
            .update_or_insert_component(db_entity, component)
            .unwrap();
    }
}
