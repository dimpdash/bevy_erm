use bevy_ecs::{component::Component, prelude::*, system::SystemParam};
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use sqlx::{database::{HasArguments, HasValueRef}, Type};
use futures::executor::block_on;
use async_trait::async_trait;


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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Component, Debug, sqlx::FromRow)]
#[sqlx(default)]
pub struct DatabaseEntity {
    pub id: i64,

    // Whether the entity has been persisted to the database ever
    // When creating an entity it will only be in memory and not have
    // been entered into the database yet, so will need to be inserted instead
    // updated
    pub persisted: bool
}

impl Default for DatabaseEntity {
    fn default() -> Self {
        DatabaseEntity {
            id: 0,
            persisted: true,
        }
    }
}

// stop type warning
pub type Pool = sqlx::Pool<sqlx::Sqlite>;

#[async_trait]
pub trait DatabaseQueryInfo: Sized {
    type Component: Component + Default;
    type Database: DatabaseResource;
    type Index: IndexInfo;

    fn get_component<'c, E>(conn: E, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;
    async fn update_component<'c, E>(tr : E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()> 
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;
    
    async fn insert_component<'c, E>(tr : E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()> 
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;
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
    fn get_internal(&mut self, db_entity : &DatabaseEntity) -> Entity {
        let mut conn = self.db.get_transaction();
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
                        let db_component = I::get_component(&mut *conn, db_entity).unwrap();
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
                let component = I::get_component(&mut *conn, db_entity).unwrap();
                unsafe {
                    let w = self.world.world_mut();
                    let entity = w.spawn((DatabaseEntity{id: db_entity.id, persisted: true}, component)).id();
                    entity
                }
            }
        }
    }

    pub fn get(&mut self, db_entity : &DatabaseEntity) -> Result<&I::Component, ()> {
        println!("getting component");
        let entity = self.get_internal(db_entity);

        unsafe {
            Ok(
                self.world.world().get::<I::Component>(entity).unwrap()
            )
        }
    }

    pub fn get_mut(&mut self, db_entity : &DatabaseEntity) -> Result<Mut<I::Component>, ()> {
        println!("getting mut component");
        let entity = self.get_internal(db_entity);

        unsafe {
            Ok(self.world.get_entity(entity).unwrap().get_mut::<I::Component>().unwrap())
        }
    }

    pub async fn update_or_insert_component<'c, E>(&self, tr : E, db_entity : &mut DatabaseEntity, component: &I::Component) -> Result<(), ()> 
    where 
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>
    {
        if db_entity.persisted {
            self.update_component(tr, db_entity, component).await?;
        } else {
            self.insert_component(tr, db_entity, component).await?;
        }
        return Ok(());
    }

    pub async fn update_component<'c, E>(&self, tr : E, db_entity : &DatabaseEntity, component: &I::Component) -> Result<(), ()> 
    where 
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>
    {
        I::update_component(tr, db_entity, component).await?;
        Ok(())
    }

    pub fn create_entity<B: Bundle>(&mut self, component: B) {
        unsafe {
            let w = self.world.world_mut();
            w.spawn((component, DatabaseEntity{id: self.db.get_key(), persisted: false}));
        }
    }

    pub async fn insert_component<'c, E>(&self, tr : E, db_entity : &mut DatabaseEntity, component: &I::Component) -> Result<(), ()> 
    where 
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>
    {
        I::insert_component(tr, db_entity, component).await?;
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
        if !world.contains_resource::<AnyDatabaseResource>() {
            world.init_resource::<AnyDatabaseResource>();
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


// Initially was going to have this trait to allow for implementing for different sql databases
// but the type system became too complex (for me)
// Now just use the sqlx::Sqlite database type
// Left the indirection in case I want to change it later

pub trait DatabaseResource: Resource + Default {
    fn get_connection(&self) -> &sqlx::Pool<sqlx::Sqlite>;
    fn get_transaction(&self) -> sqlx::Transaction<sqlx::Sqlite>;
    // A way to get a unique key for the database
    fn get_key(&mut self) -> i64;
}

#[derive(Resource)]
pub struct AnyDatabaseResource {
    pool: sqlx::SqlitePool,
    min_key: i64,
}

impl Default for AnyDatabaseResource {
    fn default() -> Self {
        let pool= block_on(sqlx::SqlitePool::connect("sqlite::memory:")).unwrap();

        AnyDatabaseResource {
            pool,
            min_key: 0,
        }
    }
}

impl DatabaseResource for AnyDatabaseResource {
    fn get_connection(&self) -> &sqlx::Pool<sqlx::Sqlite> {
        &self.pool
    }
    fn get_transaction(&self) -> sqlx::Transaction<sqlx::Sqlite> {
        block_on(self.pool.begin()).unwrap()
    }

    // Rather than actually querying the database for key just hold on to the last key we had to issue
    // This is a bit of a hack but it's fine for now. As POC and only considering one machine
    // It is progressing into the negatives so that instantiating any objects with positive keys will not conflict
    fn get_key(&mut self) -> i64 {
        self.min_key +=1;
        self.min_key
    }

}


