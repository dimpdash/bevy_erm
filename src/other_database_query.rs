use async_trait::async_trait;
use bevy_ecs::{component::Component, prelude::*, system::SystemParam};
use bevy_mod_index::prelude::*;
use bevy_reflect::Map;
use bevy_utils::hashbrown::HashSet;
use crate::database_resource::*;
use crate::database_entity::{DatabaseEntity, DatabaseEntityIndex};
use crate::database_resource::DatabaseResource;

// stop type warning
pub type Pool = sqlx::Pool<sqlx::Sqlite>;

#[async_trait]
pub trait DatabaseQueryInfo: Sized {
    type Item;
    type Database: DatabaseResource;
    type Index: IndexInfo;

    fn get<'a>(
        db: &ResMut<Self::Database>,
        world: &mut World,
    ) -> Result<&'a Self::Item, ()>;
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

    pub fn get(&mut self, db_entity: &DatabaseEntity) -> Result<&I::Item, ()> {
        unsafe {
            I::get(&self.db, self.world.world_mut())
        }
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

pub trait ComponentMapper {
    type Item;
    type Database: DatabaseResource;

    fn get<'c, E>(e : E, db_entity: &DatabaseEntity) -> Result<Self::Item, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;
}

pub trait DBQueryInfo {
    // the returned item
    type Item<'a>;
    type Database: DatabaseResource;
    type Mapper: ComponentMapper;

    fn get<'w>(&mut self, db: &mut Self::Database, world: &mut World, db_entity: &DatabaseEntity) -> Result<Self::Item<'w>, ()>;
}

pub struct NullMapper;
impl ComponentMapper for NullMapper {
    type Item = ();
    type Database = AnyDatabaseResource;

    fn get<'c, E>(e : E, db_entity: &DatabaseEntity) -> Result<Self::Item, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>
    {
        todo!()
    }
}


/*
 The macro takes a tuple of DBQueryInfo and creates 
 a new DBQueryInfo that returns a tuple of the items
*/
macro_rules! simple_composition_of_db_queries {
    ( $( $name:ident )+ ) => {
        impl DBQueryInfo for ($($name,)+)
        {
            type Item<'a> = ($(<$name as ComponentMapper>::Item, )+);
            type Database = AnyDatabaseResource;
            type Mapper = NullMapper;

            fn get<'w>(&mut self, db: &mut Self::Database, world: &mut World, db_entity: &DatabaseEntity) -> Result<Self::Item<'w>, ()> {
            //returns a tuple of all the gets
                Ok(($(
                    {
                        let mut r: SingleComponentRetriever<$name> = SingleComponentRetriever{
                            phantom: std::marker::PhantomData::default(),
                        };
                        r.get(db, world, db_entity)?
                    },
                )+))
            }
        }
    };
}

simple_composition_of_db_queries!{UserMapper UserMapper}


impl DBQueryInfo for (SingleComponentRetriever<UserMapper>, SingleComponentRetriever<UserMapper>) {
    type Item<'a> = (<UserMapper as ComponentMapper>::Item, <UserMapper as ComponentMapper>::Item, );
    type Database = AnyDatabaseResource;
    type Mapper = UserMapper;

    fn get<'w>(&mut self, db: &mut Self::Database, world: &mut World, db_entity: &DatabaseEntity) -> Result<Self::Item<'w>, ()> {
        Ok((
            self.0.get(db, world, db_entity)?,
            self.1.get(db, world, db_entity)?,
        ))
    }

}


pub struct UserMapper;
impl ComponentMapper for UserMapper {
    type Item = ();
    type Database = AnyDatabaseResource;

    fn get<'c, E>(e : E, db_entity: &DatabaseEntity) -> Result<Self::Item, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>
    {
        todo!()
    }
}


#[derive(Default)]
pub struct SingleComponentRetriever<Mapper> {
    phantom: std::marker::PhantomData<Mapper>,
}

impl<MyMapper : ComponentMapper> DBQueryInfo for SingleComponentRetriever<MyMapper> {
    type Item<'a> = MyMapper::Item;
    type Database = AnyDatabaseResource;
    type Mapper = MyMapper;

    fn get<'w>(&mut self, db: &mut Self::Database, world: &mut World, db_entity: &DatabaseEntity) -> Result<Self::Item<'w>, ()> {
    // retrieve the actual component using
        let db_handle = db.get_connection();
        let tr_option = &mut (*db_handle).write().unwrap().tr;
        let conn = tr_option.as_mut().unwrap();
        
        Self::Mapper::get(&mut **conn, db_entity)
        
        // database caching and all that good stuff
        
    }
}