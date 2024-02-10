use std::{any::TypeId, ops::Deref};

use bevy_ecs::{component::Component, prelude::*, system::SystemParam};
use bevy_reflect::prelude::*;
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use sqlx::{database::HasArguments, sqlite::*, Row, Transaction};
use futures::{executor::block_on, Future, StreamExt};
use bevy_mod_index::index::IndexFetchState;
use async_trait::async_trait;


pub struct DatabaseEntityIndex;
impl IndexInfo for DatabaseEntityIndex {
    type Component = DatabaseEntity;

    type Value = u32;

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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Component, Debug, Default, sqlx::FromRow)]
pub struct DatabaseEntity {
    pub id: u32
}
// stop type warning
pub type Pool<DbR> = sqlx::Pool<<DbR as DatabaseResource>::Database>;

type SqlxQueryAlias<'a, 'b, DBR> = sqlx::query::Query<'a, <DBR as DatabaseResource>::Database, <<DBR as DatabaseResource>::Database as HasArguments<'b>>::Arguments>;

#[async_trait]
pub trait DatabaseQueryInfo: Sized {
    type Component: Component + Default;
    type Database: DatabaseResource;
    type Index: IndexInfo;

    fn get_component(conn: &Pool<Self::Database>, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>;
    async fn write_component<'c, E>(tr : E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()> 
    where
        E: sqlx::Executor<'c, Database = <Self::Database as DatabaseResource>::Database>;
    
}

pub struct DatabaseQueryFetchState<'w, 's, I: DatabaseQueryInfo + 'static> {
    db_state: <ResMut<'w, I::Database> as SystemParam>::State,
    phantom: std::marker::PhantomData<&'s ()>,
}

pub struct DatabaseQuery<'w, 's, I:DatabaseQueryInfo + 'static> {
    pub db: ResMut<'w, I::Database>,
    world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>,
    phantom: std::marker::PhantomData<&'s ()>,
}

// pub type RODatabaseQueryItem<'a, I> = &'a I::Component;

impl<'w, 's, I:DatabaseQueryInfo> DatabaseQuery<'w, 's, I> {
    fn insert_component(&mut self, db_entity : &DatabaseEntity) -> Entity {
        let conn = self.db.get_connection();
        // using the database entity index
        // fetch from resource

        let val = db_entity.id;

        let mut reader = IntoSystem::into_system(move |mut index: Index<DatabaseEntityIndex>| -> HashSet<Entity> {
            index.lookup(&val)
        });
        
        let entity_set : HashSet<Entity> = unsafe {
            reader.initialize(&mut self.world.world_mut());
            reader.run((), &mut self.world.world_mut())
        };

        match entity_set.iter().next() {
            // Entity has been read into memory before
            Some(entity) => {
                match unsafe {self.world.world_mut().get::<I::Component>(*entity)} {
                    // Entity also already has the desired component
                    Some(_) => return *entity,
                    // Entity does not have the desired component (Load from database)
                    None => {
                        let db_component = I::get_component(conn, db_entity).unwrap();
                        // write the component to the entity
                        unsafe {
                            let w = self.world.world_mut();
                            w.entity_mut(*entity).insert(db_component);
                            return *entity
                        }
                    }
                    }
                },  
            // Entity not found in world
            None => {
                println!("entity not found in world for db_entity: {:?}", db_entity);
                let component = I::get_component(conn, db_entity).unwrap();
                unsafe {
                    let w = self.world.world_mut();
                    let entity = w.spawn((DatabaseEntity{id: db_entity.id}, component)).id();
                    entity
                }
            }
        }
    }

    pub fn get(&mut self, db_entity : &DatabaseEntity) -> Result<&I::Component, ()> {
        println!("getting component");
        let entity = self.insert_component(db_entity);

        unsafe {
            Ok(
                self.world.world().get::<I::Component>(entity).unwrap()
            )
        }
    }

    pub fn get_mut(&mut self, db_entity : &DatabaseEntity) -> Result<Mut<I::Component>, ()> {
        println!("getting mut component");
        let entity = self.insert_component(db_entity);

        unsafe {
            Ok(self.world.get_entity(entity).unwrap().get_mut::<I::Component>().unwrap())
        }
    }

    pub async fn write<'c, E>(&self, tr : E, db_entity : &DatabaseEntity, component: &I::Component) -> Result<(), ()> 
    where 
        E: sqlx::Executor<'c, Database = <I::Database as DatabaseResource>::Database>
    {
        I::write_component(tr, db_entity, component).await;

        Ok(())
    }
}

unsafe impl<'w, 's, I:DatabaseQueryInfo> SystemParam for DatabaseQuery<'w, 's, I>
    where I: DatabaseQueryInfo + 'static
{
    type State = DatabaseQueryFetchState<'static, 'static, I>;

    type Item<'_w, '_s> = DatabaseQuery<'_w, '_s, I>;

    fn init_state(world: &mut World, system_meta: &mut bevy_ecs::system::SystemMeta) -> Self::State {
        // https://github.com/chrisjuchem/bevy_mod_index/blob/15e9b4c9bbf26d4fc087ce056b07d1312464de2f/src/index.rs#L108
        if !world.contains_resource::<SqliteDatabaseResource>() {
            world.init_resource::<SqliteDatabaseResource>();
        }

        let index_state = <Index<DatabaseEntityIndex> as SystemParam>::init_state(world, system_meta);


        DatabaseQueryFetchState {
            db_state: <ResMut<'w, I::Database>>::init_state(world, system_meta),
            phantom: std::marker::PhantomData,
        }
    }

    unsafe fn get_param<'w2, 's2>(
        state: &'s2 mut Self::State,
        system_meta: &bevy_ecs::system::SystemMeta,
        world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w2>,
        change_tick: bevy_ecs::component::Tick,
    ) -> Self::Item<'w2, 's2> 
    {

        let db_query = DatabaseQuery {
            db: <ResMut<'w2, I::Database>>::get_param(
                &mut state.db_state,
                system_meta,
                world,
                change_tick),
            world: world,
            phantom: std::marker::PhantomData,
        
        };

        db_query
    }
}


pub trait DatabaseResource: Resource + Default {
    type Database: sqlx::Database;
    fn get_connection(&self) -> &sqlx::Pool<Self::Database>;
}

#[derive(Resource)]
pub struct SqliteDatabaseResource {
    pool: SqlitePool
}

impl Default for SqliteDatabaseResource {
    fn default() -> Self {
        let pool= block_on(SqlitePool::connect("sqlite::memory:")).unwrap();

        SqliteDatabaseResource {
            pool
        }
    }
}

impl DatabaseResource for SqliteDatabaseResource {
    type Database = Sqlite;
    fn get_connection(&self) -> &sqlx::Pool<Self::Database> {
        &self.pool
    }

}


