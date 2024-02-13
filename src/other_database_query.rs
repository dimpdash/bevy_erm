use bevy_ecs::{component::Component, prelude::*, system::SystemParam};
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use crate::database_resource::*;
use crate::database_entity::{DatabaseEntity, DatabaseEntityIndex};
use crate::database_resource::DatabaseResource;

pub trait DBQueryInfo {
    // the returned item
    type Item<'a>;
    type Database: DatabaseResource;
    type Mapper: ComponentMapper;

    fn get<'w>(db: &mut Self::Database, world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>, db_entity: &DatabaseEntity) -> Result<Self::Item<'w>, ()>;
}



pub type DBQueryItem<'a, Q> = <Q as DBQueryInfo>::Item<'a>;

pub struct QueryFetchState<'w, 's, I: DBQueryInfo + 'static> {
    db_state: <ResMut<'w, I::Database> as SystemParam>::State,
    phantom: std::marker::PhantomData<&'s ()>,
}

pub struct Query<'world, 'state, Q: DBQueryInfo>  {
    // world and state will be needed later
    db : ResMut<'world, <Q as DBQueryInfo>::Database>,
    world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'world>,
    phantom2: std::marker::PhantomData<&'state ()>,
}

// So query can be constructed by the system
unsafe impl<'w, 's, I: DBQueryInfo> SystemParam for Query<'w, 's, I>
where
    I: DBQueryInfo + 'static,
{
    type State = QueryFetchState<'static, 'static, I>;

    type Item<'world, 'state> = Query<'world, 'state, I>;

    fn init_state(world: &mut World, system_meta: &mut bevy_ecs::system::SystemMeta) -> Self::State {
        // https://github.com/chrisjuchem/bevy_mod_index/blob/15e9b4c9bbf26d4fc087ce056b07d1312464de2f/src/index.rs#L108
        if !world.contains_resource::<AnyDatabaseResource>() {
            world.init_resource::<AnyDatabaseResource>();
        }

        QueryFetchState {
            db_state: <ResMut<'w, <I as DBQueryInfo>::Database>>::init_state(world, system_meta),
            phantom: std::marker::PhantomData,
        }
    }

    unsafe fn get_param<'w2, 's2>(
        state: &'s2 mut Self::State,
        system_meta: &bevy_ecs::system::SystemMeta,
        world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w2>,
        change_tick: bevy_ecs::component::Tick,
    ) -> Self::Item<'w2, 's2> {
        let db_query = Query {
            db: <ResMut<'w2, <I as DBQueryInfo>::Database>>::get_param(
                &mut state.db_state,
                system_meta,
                world,
                change_tick,
            ),
            world,
            phantom2: std::marker::PhantomData,
        };

        db_query
    }
}

impl<'w, 's, Q: DBQueryInfo> Query<'w, 's, Q> {
    pub fn get(&mut self, db_entity: &DatabaseEntity) -> Result<DBQueryItem<'_, Q>, ()> {
        Q::get(&mut self.db, self.world, db_entity)
    }
}

pub trait ComponentMapper {
    type Item;

    fn get<'c, E>(e : E, db_entity: &DatabaseEntity) -> Result<Self::Item, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;
}

// To satisfy the type system when a DBQueryInfo is composed of other DBQueryInfos
pub struct NullMapper;
impl ComponentMapper for NullMapper {
    type Item = ();

    fn get<'c, E>(_e : E, _db_entity: &DatabaseEntity) -> Result<Self::Item, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>
    {
        todo!()
    }
}

// Let a Component Mapper be wrapped by a SingleComponentRetriever
// when being taking in as a DBQueryInfo
impl<A: ComponentMapper> DBQueryInfo for A
    where  <A as ComponentMapper>::Item: Component
{
    type Item<'a> = &'a <A as ComponentMapper>::Item;
    type Database = AnyDatabaseResource;
    type Mapper = NullMapper;

    fn get<'w>(db: &mut Self::Database, world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>, db_entity: &DatabaseEntity) -> Result<Self::Item<'w>, ()> {
    //returns a tuple of all the gets
        Ok(
            {
                SingleComponentRetriever::<A, Self::Database>::get(db, world, db_entity)?
            },
        )
    }
}

/*
 Like above but for tuples of DBQueryInfo
 The macro takes a tuple of DBQueryInfo and creates 
 a new DBQueryInfo that returns a tuple of the items
*/
macro_rules! simple_composition_of_db_queries {
    ( $( $name:ident )+ ) => {
        impl<$($name: ComponentMapper, )+> DBQueryInfo for ($($name,)+)
            where $(<$name as ComponentMapper>::Item: Component, )+
        {
            type Item<'a> = ($(&'a <$name as ComponentMapper>::Item, )+);
            type Database = AnyDatabaseResource;
            type Mapper = NullMapper;

            fn get<'w>(db: &mut Self::Database, world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>, db_entity: &DatabaseEntity) -> Result<Self::Item<'w>, ()> {
            //returns a tuple of all the gets
                Ok(($(
                    {
                        SingleComponentRetriever::<$name, Self::Database>::get(db, world, db_entity)?
                    },
                )+))
            }
        }
    };
}



// Create a simple composition of DBQueryInfo for tuples of length 1 to 10
// Allows DBQueryInfo to be composed of other DBQueryInfo
// eg. DBQuery<(User, Item)>
simple_composition_of_db_queries!{A}
simple_composition_of_db_queries!{A B}
simple_composition_of_db_queries!{A B C}
simple_composition_of_db_queries!{A B C D}
simple_composition_of_db_queries!{A B C D E}
simple_composition_of_db_queries!{A B C D E F}
simple_composition_of_db_queries!{A B C D E F G}
simple_composition_of_db_queries!{A B C D E F G H}
simple_composition_of_db_queries!{A B C D E F G H I}
simple_composition_of_db_queries!{A B C D E F G H I J}
simple_composition_of_db_queries!{A B C D E F G H I J K}

pub struct UserMapper;
impl ComponentMapper for UserMapper {
    type Item = i64;

    fn get<'c, E>(_e : E, _db_entity: &DatabaseEntity) -> Result<Self::Item, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>
    {
        todo!()
    }
}


#[derive(Default)]
pub struct SingleComponentRetriever<Mapper, DatabaseResource> {
    phantom: std::marker::PhantomData<(Mapper, DatabaseResource)>,
}

impl <MyMapper : ComponentMapper> SingleComponentRetriever<MyMapper, AnyDatabaseResource>
where <MyMapper as ComponentMapper>::Item: Component
{

    fn get_internal<'w>(
        db: &AnyDatabaseResource, 
        world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>, 
        db_entity: &DatabaseEntity,
        component_preloaded: Option<<MyMapper as ComponentMapper>::Item>,
    ) -> Entity {
        // let conn = self.db.get_transaction();
        // using the database entity index
        // fetch from resource
        let db_handle = db.get_connection();
        let tr_option = &mut (*db_handle).write().unwrap().tr;
        let conn = tr_option.as_mut().unwrap();

        let val = db_entity.id;

        let mut reader = IntoSystem::into_system(
            move |mut index: Index<DatabaseEntityIndex>| -> HashSet<Entity> { index.lookup(&val) },
        );

        let entity_set: HashSet<Entity> = unsafe {
            reader.initialize(world.world_mut());
            reader.run((), world.world_mut())
        };

        match entity_set.iter().next() {
            // Entity has been read into memory before
            Some(entity) => {
                match unsafe { world.world_mut().get::<<MyMapper as ComponentMapper>::Item>(*entity) } {
                    // Entity also already has the desired component
                    Some(_) => *entity,
                    // Entity does not have the desired component (Load from database)
                    None => {
                        let db_component = match component_preloaded {
                            Some(component) => component,
                            None => MyMapper::get(&mut **conn, db_entity).unwrap(),
                        };
                        // write the component to the entity
                        unsafe {
                            let w = world.world_mut();
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
                    None => MyMapper::get(&mut **conn, db_entity).unwrap(),
                };
                unsafe {
                    let w = world.world_mut();
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

}

impl<MyMapper : ComponentMapper> DBQueryInfo for SingleComponentRetriever<MyMapper, AnyDatabaseResource> 
where <MyMapper as ComponentMapper>::Item: Component
{
    type Item<'a> = &'a MyMapper::Item;
    type Database = AnyDatabaseResource;
    type Mapper = MyMapper;


    fn get<'w>(db: &mut Self::Database, world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>, db_entity: &DatabaseEntity) -> Result<Self::Item<'w>, ()> {
        let entity = Self::get_internal(db, world, db_entity, None);
        
        unsafe { Ok(world.world().get::<<MyMapper as ComponentMapper>::Item>(entity).unwrap()) }
    }
}

