use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use bevy_ecs::{component::Component, prelude::*, system::SystemParam};
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use futures::executor::block_on;
use sqlx::Transaction;

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
pub struct Persisted(bool);

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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Component, Debug, Default, sqlx::FromRow)]
pub struct DatabaseEntity {
    pub id: i64,

    // Whether the entity has been persisted to the database ever
    // When creating an entity it will only be in memory and not have
    // been entered into the database yet, so will need to be inserted instead
    // updated
    #[sqlx(skip)]
    pub persisted: Persisted,
}

// stop type warning
pub type Pool = sqlx::Pool<sqlx::Sqlite>;

#[async_trait]
pub trait DatabaseQueryInfo: Sized {
    type Component: Component
        + Default
        + for<'a> sqlx::FromRow<'a, sqlx::sqlite::SqliteRow>
        + Unpin
        + std::fmt::Debug;
    type Database: DatabaseResource;
    type Index: IndexInfo;

    fn get_component<'c, E>(conn: E, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;
    async fn update_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;

    async fn insert_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;

    fn table_name() -> Result<&'static str, ()>;
}

pub struct DatabaseQueryFetchState<'w, 's, I: DatabaseQueryInfo + 'static> {
    db_state: <ResMut<'w, I::Database> as SystemParam>::State,
    phantom: std::marker::PhantomData<&'s ()>,
}

pub struct DatabaseQuery<'w, 's, I: DatabaseQueryInfo + 'static> {
    pub db: ResMut<'w, I::Database>,
    world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>,
    phantom: std::marker::PhantomData<&'s ()>,
}

// pub type RODatabaseQueryItem<'a, I> = &'a I::Component;

pub trait QueryFn<Component>:
    FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, Component)>, ()>
{
}

impl<'w, 's, I: DatabaseQueryInfo> DatabaseQuery<'w, 's, I> {
    pub fn load_entities_for_components(
        &mut self,
        get_comp_from_db: impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, I::Component)>, ()>,
    ) -> Result<Vec<Entity>, ()> {
        // let conn = self.db.get_transaction();
        // using the database entity index
        // fetch from resource

        let components = {
            let db_handle = self.db.get_connection();
            let tr_option = &mut (*db_handle).write().unwrap().tr;
            let conn = tr_option.as_mut().unwrap();

            get_comp_from_db(&mut **conn)?
        };

        let entities = components
            .into_iter()
            .map(|(db_entity, component)| self.get_internal(&db_entity, Some(component)))
            .collect::<Vec<Entity>>();

        Ok(entities)
    }

    pub fn load_components(
        &mut self,
        get_comp_from_db: impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, I::Component)>, ()>,
    ) -> Result<Vec<&'w I::Component>, ()> {
        Ok(self
            .load_entities_for_components(get_comp_from_db)?
            .into_iter()
            .map(|entity| unsafe { self.world.world().get::<I::Component>(entity).unwrap() })
            .collect())
    }

    pub fn load_components_mut(
        &mut self,
        get_comp_from_db: impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, I::Component)>, ()>,
    ) -> Result<Vec<Mut<I::Component>>, ()> {
        Ok(self
            .load_entities_for_components(get_comp_from_db)?
            .into_iter()
            .map(|entity| unsafe {
                self.world
                    .get_entity(entity)
                    .unwrap()
                    .get_mut::<I::Component>()
                    .unwrap()
            })
            .collect())
    }

    fn get_internal(
        &mut self,
        db_entity: &DatabaseEntity,
        component_preloaded: Option<I::Component>,
    ) -> Entity {
        // let conn = self.db.get_transaction();
        // using the database entity index
        // fetch from resource
        let db_handle = self.db.get_connection();
        let tr_option = &mut (*db_handle).write().unwrap().tr;
        let conn = tr_option.as_mut().unwrap();

        let val = db_entity.id;

        let mut reader = IntoSystem::into_system(
            move |mut index: Index<DatabaseEntityIndex>| -> HashSet<Entity> { index.lookup(&val) },
        );

        let entity_set: HashSet<Entity> = unsafe {
            reader.initialize(self.world.world_mut());
            reader.run((), self.world.world_mut())
        };

        match entity_set.iter().next() {
            // Entity has been read into memory before
            Some(entity) => {
                match unsafe { self.world.world_mut().get::<I::Component>(*entity) } {
                    // Entity also already has the desired component
                    Some(_) => *entity,
                    // Entity does not have the desired component (Load from database)
                    None => {
                        let db_component = match component_preloaded {
                            Some(component) => component,
                            None => I::get_component(&mut **conn, db_entity).unwrap(),
                        };
                        // write the component to the entity
                        unsafe {
                            let w = self.world.world_mut();
                            w.entity_mut(*entity).insert(db_component);
                            *entity
                        }
                    }
                }
            }
            // Entity not found in world
            None => {
                println!("entity not found in world for db_entity: {:?}", db_entity);

                let component = match component_preloaded {
                    Some(component) => component,
                    None => I::get_component(&mut **conn, db_entity).unwrap(),
                };
                unsafe {
                    let w = self.world.world_mut();
                    let entity = w
                        .spawn((
                            DatabaseEntity {
                                id: db_entity.id,
                                persisted: true.into(),
                            },
                            component,
                        ))
                        .id();
                    entity
                }
            }
        }
    }

    pub fn get(&mut self, db_entity: &DatabaseEntity) -> Result<&I::Component, ()> {
        println!("getting component");
        let entity = self.get_internal(db_entity, None);

        unsafe { Ok(self.world.world().get::<I::Component>(entity).unwrap()) }
    }

    pub fn get_mut(&mut self, db_entity: &DatabaseEntity) -> Result<Mut<I::Component>, ()> {
        println!("getting mut component");
        let entity = self.get_internal(db_entity, None);

        unsafe {
            Ok(self
                .world
                .get_entity(entity)
                .unwrap()
                .get_mut::<I::Component>()
                .unwrap())
        }
    }

    pub async fn update_or_insert_component<'c, E>(
        &self,
        tr: E,
        db_entity: &DatabaseEntity,
        component: &I::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        if db_entity.persisted.into() {
            self.update_component(tr, db_entity, component).await?;
        } else {
            self.insert_component(tr, db_entity, component).await?;
        }
        Ok(())
    }

    pub async fn update_component<'c, E>(
        &self,
        tr: E,
        db_entity: &DatabaseEntity,
        component: &I::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        I::update_component(tr, db_entity, component).await?;
        Ok(())
    }

    pub fn create_entity<B: Bundle>(&mut self, component: B) {
        unsafe {
            let w = self.world.world_mut();
            w.spawn((
                component,
                DatabaseEntity {
                    id: self.db.get_key(),
                    persisted: false.into(),
                },
            ));
        }
    }

    pub async fn insert_component<'c, E>(
        &self,
        tr: E,
        db_entity: &DatabaseEntity,
        component: &I::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        I::insert_component(tr, db_entity, component).await?;
        Ok(())
    }
}

unsafe impl<'w, 's, I: DatabaseQueryInfo> SystemParam for DatabaseQuery<'w, 's, I>
where
    I: DatabaseQueryInfo + 'static,
{
    type State = DatabaseQueryFetchState<'static, 'static, I>;

    type Item<'_w, '_s> = DatabaseQuery<'_w, '_s, I>;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy_ecs::system::SystemMeta,
    ) -> Self::State {
        // https://github.com/chrisjuchem/bevy_mod_index/blob/15e9b4c9bbf26d4fc087ce056b07d1312464de2f/src/index.rs#L108
        if !world.contains_resource::<AnyDatabaseResource>() {
            world.init_resource::<AnyDatabaseResource>();
        }

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
    ) -> Self::Item<'w2, 's2> {
        let db_query = DatabaseQuery {
            db: <ResMut<'w2, I::Database>>::get_param(
                &mut state.db_state,
                system_meta,
                world,
                change_tick,
            ),
            world,
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
    fn get_connection(&self) -> Arc<RwLock<DatabaseHandle>>;
    // A way to get a unique key for the database
    fn get_key(&mut self) -> i64;
}

#[derive(Debug)]
pub struct DatabaseHandle {
    pub pool: sqlx::SqlitePool,
    // Currently on handle one transaction at a time
    // IMPROVEMNET: Use a vec of transactions to allow for multiple transactions at once
    pub tr: Option<Transaction<'static, sqlx::Sqlite>>,
}

#[derive(Resource, Debug)]
pub struct AnyDatabaseResource {
    min_key: i64,
    db: Arc<RwLock<DatabaseHandle>>,
}

impl Default for AnyDatabaseResource {
    fn default() -> Self {
        let pool = block_on(sqlx::SqlitePool::connect("sqlite::memory:")).unwrap();
        let tr = Some(block_on(pool.begin()).unwrap());
        let db = Arc::new(RwLock::new(DatabaseHandle { pool, tr }));
        AnyDatabaseResource { min_key: 0, db }
    }
}

unsafe impl Sync for DatabaseHandle {}
unsafe impl Send for DatabaseHandle {}

unsafe impl Sync for AnyDatabaseResource {}

impl DatabaseResource for AnyDatabaseResource {
    fn get_connection(&self) -> Arc<RwLock<DatabaseHandle>> {
        self.db.clone()
    }

    // Rather than actually querying the database for key just hold on to the last key we had to issue
    // This is a bit of a hack but it's fine for now. As POC and only considering one machine
    // It is progressing into the negatives so that instantiating any objects with positive keys will not conflict
    fn get_key(&mut self) -> i64 {
        self.min_key -= 1;
        self.min_key
    }
}

pub fn flush_component_to_db<T: DatabaseQueryInfo>(
    query: Query<(&DatabaseEntity, &T::Component)>,
    db_query: DatabaseQuery<T>,
) {
    let db_handle = db_query.db.get_connection();
    let tr_option = &mut (*db_handle).write().unwrap().tr;
    let tr = tr_option.as_mut().unwrap();

    block_on(async {
        for (db_entity, component) in query.iter() {
            println!("flushing component to db");
            println!("db_entity: {:?}, component {:?}", db_entity, component);
            db_query
                .update_or_insert_component(&mut **tr, db_entity, component)
                .await
                .unwrap();
        }
    });
}
