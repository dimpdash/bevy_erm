use async_trait::async_trait;
use bevy_ecs::{component::Component, prelude::*, system::SystemParam};
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use crate::database_resource::*;
use crate::database_entity::{DatabaseEntity, DatabaseEntityIndex};
use crate::database_resource::DatabaseResource;

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

    async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntity) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;
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
        get_comp_from_db: impl FnOnce(
            &mut sqlx::SqliteConnection,
        ) -> Result<Vec<(DatabaseEntity, I::Component)>, ()>,
    ) -> Result<Vec<Entity>, ()> {
        // let conn = self.db.get_transaction();
        // using the database entity index
        // fetch from resource

        let components = {
            let db_handle = self.db.get_connection();
            let tr_option = &mut (*db_handle).write().unwrap().tr;
            let conn = tr_option.as_mut().unwrap();

            get_comp_from_db(conn)?
        };

        let entities = components
            .into_iter()
            .map(|(db_entity, component)| self.get_internal(&db_entity, Some(component)))
            .collect::<Vec<Entity>>();

        Ok(entities)
    }

    pub fn load_components(
        &mut self,
        get_comp_from_db: impl FnOnce(
            &mut sqlx::SqliteConnection,
        ) -> Result<Vec<(DatabaseEntity, I::Component)>, ()>,
    ) -> Result<Vec<&'w I::Component>, ()> {
        Ok(self
            .load_entities_for_components(get_comp_from_db)?
            .into_iter()
            .map(|entity| unsafe { self.world.world().get::<I::Component>(entity).unwrap() })
            .collect())
    }

    pub fn load_components_and_entity(
        &mut self,
        get_comp_from_db: impl FnOnce(
            &mut sqlx::SqliteConnection,
        ) -> Result<Vec<(DatabaseEntity, I::Component)>, ()>,
    ) -> Result<Vec<(Entity, &'w I::Component)>, ()> {
        Ok(self
            .load_entities_for_components(get_comp_from_db)?
            .into_iter()
            .map(|entity| {
                (
                    entity,
                    unsafe { self.world.world().get::<I::Component>(entity).unwrap() },
                )
            })
            .collect())
    }


    pub fn load_components_mut(
        &mut self,
        get_comp_from_db: impl FnOnce(
            &mut sqlx::SqliteConnection,
        ) -> Result<Vec<(DatabaseEntity, I::Component)>, ()>,
    ) -> Result<Vec<Mut<I::Component>>, ()> {
        Ok(self
            .load_entities_for_components(get_comp_from_db)?
            .into_iter()
            .map(|entity| unsafe {
                // set entity to dirty
                self.world
                    .get_entity(entity)
                    .unwrap()
                    .get_mut::<DatabaseEntity>()
                    .unwrap()
                    .dirty = true;

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
                                dirty: false,
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
            // set entity to dirty
            self.world
                .get_entity(entity)
                .unwrap()
                .get_mut::<DatabaseEntity>()
                .unwrap()
                .dirty = true;

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
            if db_entity.dirty {
                self.update_component(tr, db_entity, component).await?;
            }
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
                    dirty: false,
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

    pub async fn delete_component<'c, E>(&self, db_entity: &DatabaseEntity) -> Result<(), ()> {
        let db_handle = self.db.get_connection();
        let tr_option = &mut (*db_handle).write().unwrap().tr;
        let tr = tr_option.as_mut().unwrap();

        I::delete_component(&mut **tr, db_entity).await?;
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



pub type DBQueryItem<'a, Q> = <Q as DBQueryInfo>::Item<'a>;

struct Query<'world, 'state, Q: DBQueryInfo>  {
    db : <Q as DBQueryInfo>::Database,
    // world and state will be needed later
    phantom: std::marker::PhantomData<&'world ()>,
    phantom2: std::marker::PhantomData<&'state ()>,
}

impl<'w, 's, Q: DBQueryInfo> Query<'w, 's, Q> {
    fn get(&self) -> Result<DBQueryItem<'_, Q>, ()> {
        unimplemented!()
    }
}


// impl DBQueryInfo for DatabaseEntity {
//     type Item<'a> = &'a DatabaseEntity;
//     type Database = AnyDatabaseResource;

//     fn get<'w>(&mut self) -> Result<Self::Item<'w>, ()> {
//         todo!()
//     }
// }


pub trait DBQueryInfo {
    // the returned item
    type Item<'a>;
    type Database: DatabaseResource;

    fn get<'w>(&mut self, db: &mut Self::Database) -> Result<Self::Item<'w>, ()>;
}



impl DBQueryInfo for DatabaseEntity {
    type Item<'a> = &'a DatabaseEntity;
    type Database = AnyDatabaseResource;

    fn get<'w>(&mut self, db: &mut Self::Database) -> Result<Self::Item<'w>, ()> {
        todo!()
    }
}

/*
 The macro takes a tuple of DBQueryInfo and creates 
 a new DBQueryInfo that returns a tuple of the items
*/
macro_rules! simple_composition_of_db_queries {
    ( $( $name:ident )+ ) => {
        impl<$($name: DBQueryInfo),+> DBQueryInfo for ($($name,)+)
        {
            type Item<'a> = ($($name::Item<'a>,)+);
            type Database = <DatabaseEntity as DBQueryInfo>::Database;

            fn get<'w>(&mut self, db: &mut Self::Database) -> Result<Self::Item<'w>, ()> {
                //returns a tuple of all the gets
                let ($($name,)+) = self;
                Ok(($($name.get(db)?, )+))
            }
        }
    };
}


simple_composition_of_db_queries! { DatabaseEntity }