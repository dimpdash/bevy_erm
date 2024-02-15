use crate::*;
use async_trait::async_trait;
use bevy_ecs::query::WorldQuery;
use bevy_ecs::{
    component::Component, prelude::*, system::SystemParam,
    world::unsafe_world_cell::UnsafeWorldCell,
};
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use casey::lower;
use futures::executor::block_on;

type QueryItem<'a, Q> = <Q as DBQueryInfo>::Item<'a>;
type ROQueryItem<'a, Q> = <Q as DBQueryInfo>::ReadOnlyItem<'a>;

pub trait ReturnSelector<'w> {
    type ReturnItem;

    fn load_components_from_entities(
        world: UnsafeWorldCell<'w>,
        entities: Vec<Entity>,
    ) -> Vec<Self::ReturnItem>;
}

pub type DatabaseConnection<'a, D> = <D as DatabaseResource>::DatabaseConnection<'a>;

pub trait DBQueryInfo {
    // the returned item
    type Item<'a>;
    type ReadOnlyItem<'a>;
    type DerefItem;
    // type ReadOnly: ReadOnlyDBQueryInfo<Database = Self::Database, Mapper = Self::Mapper>;
    type Database: DatabaseResource;
    type Mapper: ComponentMapper;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()>;
    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()>;
    fn update_component<'w>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()>;
    fn insert_component<'w>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()>;
    fn load_components<'w, R: ReturnSelector<'w>>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseConnection<Self::Database>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>;
    fn create(
        db: &Self::Database,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()>;
}

pub type DBQueryItem<'a, Q> = <Q as DBQueryInfo>::Item<'a>;

pub struct QueryFetchState<'w, 's, I: DBQueryInfo + 'static> {
    db_state: <Res<'w, I::Database> as SystemParam>::State,
    phantom: std::marker::PhantomData<&'s ()>,
}

pub struct DatabaseQuery<'world, 'state, Q: DBQueryInfo> {
    // world and state will be needed later
    db: Res<'world, <Q as DBQueryInfo>::Database>,
    world: UnsafeWorldCell<'world>,
    phantom2: std::marker::PhantomData<&'state ()>,
}

// So query can be constructed by the system
unsafe impl<'w, 's, I: DBQueryInfo> SystemParam for DatabaseQuery<'w, 's, I>
where
    I: DBQueryInfo + 'static,
{
    type State = QueryFetchState<'static, 'static, I>;

    type Item<'world, 'state> = DatabaseQuery<'world, 'state, I>;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy_ecs::system::SystemMeta,
    ) -> Self::State {
        // https://github.com/chrisjuchem/bevy_mod_index/blob/15e9b4c9bbf26d4fc087ce056b07d1312464de2f/src/index.rs#L108
        if !world.contains_resource::<AnyDatabaseResource>() {
            world.init_resource::<AnyDatabaseResource>();
        }

        QueryFetchState {
            db_state: <Res<'w, <I as DBQueryInfo>::Database>>::init_state(world, system_meta),
            phantom: std::marker::PhantomData,
        }
    }

    unsafe fn get_param<'w2, 's2>(
        state: &'s2 mut Self::State,
        system_meta: &bevy_ecs::system::SystemMeta,
        world: UnsafeWorldCell<'w2>,
        change_tick: bevy_ecs::component::Tick,
    ) -> Self::Item<'w2, 's2> {
        let db_query = DatabaseQuery {
            db: <Res<'w2, <I as DBQueryInfo>::Database>>::get_param(
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

pub trait DatabaseEntityWithRequest {
    fn request(&self) -> &RequestId;
    fn id(&self) -> &DatabaseEntityId;
}

impl<'w, 's, Q: DBQueryInfo> DatabaseQuery<'w, 's, Q> {
    pub fn get<D: DatabaseEntityWithRequest>(
        &self,
        db_entity: &D,
    ) -> Result<Q::ReadOnlyItem<'_>, ()> {
        Q::get(&self.db, self.world, db_entity)
    }

    pub fn get_mut<D: DatabaseEntityWithRequest>(&self, db_entity: &D) -> Result<Q::Item<'_>, ()> {
        Q::get_mut(&self.db, self.world, db_entity)
    }

    pub fn update_component(
        &self,
        db_entity: &DatabaseEntity,
        component: Q::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        Q::update_component(&self.db, self.world, db_entity, component)
    }

    pub fn insert_component(
        &self,
        db_entity: &DatabaseEntity,
        component: Q::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        Q::insert_component(&self.db, self.world, db_entity, component)
    }

    pub fn load_components<R: ReturnSelector<'w>>(
        &self,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseConnection<Q::Database>,
        ) -> Result<Vec<(DatabaseEntity, Q::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        Q::load_components::<R>(&self.db, self.world, request, get_comp_from_db)
    }

    pub fn create(&self, component: Q::DerefItem, request: RequestId) -> Result<(), ()> {
        Q::create(&self.db, self.world, component, request)
    }

    pub fn update_or_insert_component(
        &self,
        db_entity: &DatabaseEntity,
        component: Q::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        if db_entity.persisted.into() {
            if db_entity.dirty {
                Q::update_component(&self.db, self.world, db_entity, component)
            } else {
                Ok(())
            }
        } else {
            Q::insert_component(&self.db, self.world, db_entity, component)
        }
    }
}

#[async_trait]
pub trait ComponentMapper {
    type Component;

    async fn get<'c, E>(e: E, db_entity: &DatabaseEntityId) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;

    async fn update_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;

    async fn insert_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>;
}

#[derive(Component)]
pub struct NullComponent();

impl RequestIdIndexInfo for NullComponent {}

// To satisfy the type system when a DBQueryInfo is composed of other DBQueryInfos
impl IndexInfo for NullComponent {
    type Component = NullComponent;
    type Value = RequestId;
    type Storage = NoStorage<Self>;

    fn value(_c: &Self::Component) -> Self::Value {
        RequestId(generational_arena::Index::from_raw_parts(0, 0))
    }
}

pub struct NullMapper;
#[async_trait]
impl ComponentMapper for NullMapper {
    type Component = NullComponent;

    async fn get<'c, E>(_e: E, _db_entity: &DatabaseEntityId) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        unimplemented!()
    }

    async fn update_component<'c, E>(
        _tr: E,
        _db_entity: &DatabaseEntityId,
        _component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        unimplemented!()
    }

    async fn insert_component<'c, E>(
        _tr: E,
        _db_entity: &DatabaseEntityId,
        _component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        unimplemented!()
    }
}

// Used to help speicfy whether the returned component is read only or mutable
pub trait ReadMarker: DBQueryInfo {}

impl<T: ComponentMapper> ReadMarker for &T where <T as ComponentMapper>::Component: Component {}

impl<T: ComponentMapper> DBQueryInfo for &T
where
    <T as ComponentMapper>::Component: Component,
{
    type Item<'a> = &'a <T as ComponentMapper>::Component;
    type ReadOnlyItem<'a> = &'a <T as ComponentMapper>::Component;
    type DerefItem = <T as ComponentMapper>::Component;
    type Database = AnyDatabaseResource;
    type Mapper = NullMapper;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()> {
        SingleComponentRetriever::<T, Self::Database>::get(db, world, db_entity)
    }

    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()> {
        SingleComponentRetriever::<T, Self::Database>::get(db, world, db_entity)
    }

    fn update_component<'w>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, Self::Database>::update_component(
            db, world, db_entity, component,
        )
    }

    fn insert_component<'w>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, Self::Database>::insert_component(
            db, world, db_entity, component,
        )
    }

    fn load_components<'w, R: ReturnSelector<'w>>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseConnection<Self::Database>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        SingleComponentRetriever::<T, Self::Database>::load_components::<R>(
            db,
            world,
            request,
            get_comp_from_db,
        )
    }

    fn create(
        db: &Self::Database,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, Self::Database>::create(db, world, component, request)
    }
}

impl<T: ComponentMapper> ReadMarker for &mut T where <T as ComponentMapper>::Component: Component {}

impl<T: ComponentMapper> DBQueryInfo for &mut T
where
    <T as ComponentMapper>::Component: Component,
{
    type Item<'a> = Mut<'a, <T as ComponentMapper>::Component>;
    type ReadOnlyItem<'a> = &'a <T as ComponentMapper>::Component;
    type DerefItem = <T as ComponentMapper>::Component;
    type Database = AnyDatabaseResource;
    type Mapper = NullMapper;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()> {
        SingleComponentRetriever::<T, Self::Database>::get(db, world, db_entity)
    }

    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()> {
        SingleComponentRetriever::<T, Self::Database>::get_mut(db, world, db_entity)
    }

    fn update_component<'w>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, Self::Database>::update_component(
            db, world, db_entity, component,
        )
    }

    fn insert_component<'w>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, Self::Database>::insert_component(
            db, world, db_entity, component,
        )
    }

    fn load_components<'w, R: ReturnSelector<'w>>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseConnection<Self::Database>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        SingleComponentRetriever::<T, Self::Database>::load_components::<R>(
            db,
            world,
            request,
            get_comp_from_db,
        )
    }

    fn create(
        db: &Self::Database,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, Self::Database>::create(db, world, component, request)
    }
}

/*
 Like above but for tuples of DBQueryInfo
 The macro takes a tuple of DBQueryInfo and creates
 a new DBQueryInfo that returns a tuple of the items
*/
macro_rules! simple_composition_of_db_queries {
    ( $( $name:ident )* ) => {
        // include this Head Z so that can specify that the rest of them have the same database
        // looks quite ugly but it works
        impl<Z: ReadMarker, $($name: ReadMarker<Database = <Z as DBQueryInfo>::Database>, )*> DBQueryInfo for (Z, $($name,)*)
        {
            type Item<'a> = (QueryItem<'a, Z>, $(QueryItem<'a, $name>, )*);
            type ReadOnlyItem<'a> = (ROQueryItem<'a, Z>, $(ROQueryItem<'a, $name>, )*);
            type DerefItem = (Z::DerefItem, $($name::DerefItem,)*);
            type Database = <Z as DBQueryInfo>::Database;
            type Mapper = NullMapper;

            fn get<'w, D: DatabaseEntityWithRequest>(db: &Self::Database, world: UnsafeWorldCell<'w>, db_entity: &D) -> Result<Self::ReadOnlyItem<'w>, ()> {
            //returns a tuple of all the gets
                Ok((
                    Z::get(db, world, db_entity)?,
                    $(
                    {
                        $name::get(db, world, db_entity)?
                    },
                )*))
            }

            fn get_mut<'w, D: DatabaseEntityWithRequest>(db: &Self::Database, world: UnsafeWorldCell<'w>, db_entity: &D) -> Result<Self::Item<'w>, ()> {
                //returns a tuple of all the gets
                Ok((
                    Z::get_mut(db, world, db_entity)?,

                    $(
                    {
                        $name::get_mut(db, world, db_entity)?
                    },
                )*))
            }

            fn update_component<'w>(db: &Self::Database, world: UnsafeWorldCell<'w>, db_entity: &DatabaseEntity, component: Self::ReadOnlyItem<'w>) -> Result<(), ()> {

                let (z, $(lower!($name),)*) = component;

                Z::update_component(db, world, db_entity, z)?;
                $($name::update_component(db, world, db_entity, lower!($name))?;)*

                Ok(())
            }

            fn insert_component<'w>(db: &Self::Database, world: UnsafeWorldCell<'w>, db_entity: &DatabaseEntity, component: Self::ReadOnlyItem<'w>) -> Result<(), ()> {
                let (z, $(lower!($name),)*) = component;

                Z::insert_component(db, world, db_entity, z)?;
                $($name::insert_component(db, world, db_entity, lower!($name))?;)*

                Ok(())
            }

            // CODE SMELL: probably should split up interface to avoid this method
            fn load_components<'w, R : ReturnSelector<'w>>(
                _db: &Self::Database,
                _world: UnsafeWorldCell<'w>,
                _request: RequestId,
                _get_comp_from_db: impl FnOnce(DatabaseConnection<Self::Database>) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>
            ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
                unimplemented!()
            }

            fn create(db: &Self::Database, world: UnsafeWorldCell<'_>, component: Self::DerefItem, request: RequestId) -> Result<(), ()> {
                let (z, $(lower!($name),)*) = component;

                Z::create(db, world, z, request)?;
                $($name::create(db, world, lower!($name), request)?;)*

                Ok(())
            }
        }
    };
}

// Create a simple composition of DBQueryInfo for tuples of length 1 to 10
// Allows DBQueryInfo to be composed of other DBQueryInfo
// eg. DBQuery<(User, Item)>
// simple_composition_of_db_queries!{}
simple_composition_of_db_queries! {A}
simple_composition_of_db_queries! {A B}
//uncomment while deving to speedup compile times
// simple_composition_of_db_queries!{A B C}
// simple_composition_of_db_queries!{A B C D}
// simple_composition_of_db_queries!{A B C D E}
// simple_composition_of_db_queries!{A B C D E F}
// simple_composition_of_db_queries!{A B C D E F G}
// simple_composition_of_db_queries!{A B C D E F G H}
// simple_composition_of_db_queries!{A B C D E F G H I}
// simple_composition_of_db_queries!{A B C D E F G H I J}
// simple_composition_of_db_queries!{A B C D E F G H I J K}

#[derive(Default)]
pub struct SingleComponentRetriever<Mapper, DatabaseResource> {
    phantom: std::marker::PhantomData<(Mapper, DatabaseResource)>,
}

impl<MyMapper: ComponentMapper> SingleComponentRetriever<MyMapper, AnyDatabaseResource>
where
    <MyMapper as ComponentMapper>::Component: Component,
{
    fn get_internal<D: DatabaseEntityWithRequest>(
        db: &AnyDatabaseResource,
        world: UnsafeWorldCell<'_>,
        db_entity: &D,
        component_preloaded: Option<<MyMapper as ComponentMapper>::Component>,
    ) -> Entity {
        get_transaction!(tr, db_entity.request(), db);

        let db_entity_id = *db_entity.id();

        let mut reader = IntoSystem::into_system(
            move |mut index: Index<DatabaseEntityIndex>| -> HashSet<Entity> {
                index.lookup(&db_entity_id)
            },
        );

        let entity_set: HashSet<Entity> = unsafe {
            reader.initialize(world.world_mut());
            reader.run((), world.world_mut())
        };

        match entity_set.iter().next() {
            // Entity has been read into memory before
            Some(entity) => {
                match unsafe {
                    world
                        .world_mut()
                        .get::<<MyMapper as ComponentMapper>::Component>(*entity)
                } {
                    // Entity also already has the desired component
                    Some(_) => *entity,
                    // Entity does not have the desired component (Load from database)
                    None => {
                        let db_component = match component_preloaded {
                            Some(component) => component,
                            None => block_on(MyMapper::get(&mut **tr, db_entity.id())).unwrap(),
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
                    None => block_on(MyMapper::get(&mut **tr, db_entity.id())).unwrap(),
                };
                unsafe {
                    let w = world.world_mut();
                    let entity = w
                        .spawn((
                            DatabaseEntity {
                                id: *db_entity.id(),
                                persisted: true.into(),
                                dirty: false,
                                request: *db_entity.request(),
                            },
                            component,
                        ))
                        .id();
                    entity
                }
            }
        }
    }

    pub fn load_entities_for_components(
        db: &AnyDatabaseResource,
        world: UnsafeWorldCell<'_>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseConnection<AnyDatabaseResource>,
        ) -> Result<
            Vec<(DatabaseEntity, <MyMapper as ComponentMapper>::Component)>,
            (),
        >,
    ) -> Result<Vec<Entity>, ()> {
        let components = {
            get_transaction!(tr, request, db);
            get_comp_from_db(&mut *tr)?
        };

        let entities = components
            .into_iter()
            .map(|(db_entity, component)| {
                Self::get_internal(db, world, &db_entity, Some(component))
            })
            .collect::<Vec<Entity>>();

        Ok(entities)
    }
}

impl<'w, C: WorldQuery> ReturnSelector<'w> for C {
    type ReturnItem = C::Item<'w>;

    fn load_components_from_entities(
        world: UnsafeWorldCell<'w>,
        entities: Vec<Entity>,
    ) -> Vec<Self::ReturnItem> {
        entities
            .into_iter()
            .map(|entity| unsafe {
                let world = world.world_mut();
                let mut q = world.query::<C>();
                q.get_mut(world, entity).unwrap()
            })
            .collect()
    }
}

impl<MyMapper: ComponentMapper> DBQueryInfo
    for SingleComponentRetriever<MyMapper, AnyDatabaseResource>
where
    <MyMapper as ComponentMapper>::Component: Component,
{
    type Item<'a> = Mut<'a, MyMapper::Component>;
    type ReadOnlyItem<'a> = &'a MyMapper::Component;
    type DerefItem = MyMapper::Component;
    type Database = AnyDatabaseResource;
    type Mapper = MyMapper;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()> {
        let entity = Self::get_internal(db, world, db_entity, None);

        unsafe {
            Ok(world
                .world()
                .get::<<MyMapper as ComponentMapper>::Component>(entity)
                .unwrap())
        }
    }

    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()> {
        let entity = Self::get_internal(db, world, db_entity, None);

        unsafe {
            // set entity to dirty
            world
                .get_entity(entity)
                .unwrap()
                .get_mut::<DatabaseEntity>()
                .unwrap()
                .dirty = true;

            Ok(world
                .get_entity(entity)
                .unwrap()
                .get_mut::<<MyMapper as ComponentMapper>::Component>()
                .unwrap())
        }
    }

    fn update_component<'w>(
        db: &Self::Database,
        _world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        get_transaction!(tr, db_entity.request, db);

        block_on(MyMapper::update_component(
            &mut **tr,
            db_entity.id(),
            component,
        ))
    }

    fn insert_component<'w>(
        db: &Self::Database,
        _world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        get_transaction!(tr, db_entity.request, db);

        block_on(MyMapper::insert_component(
            &mut **tr,
            db_entity.id(),
            component,
        ))
    }

    fn load_components<'w, R: ReturnSelector<'w>>(
        db: &Self::Database,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseConnection<Self::Database>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        let entities = Self::load_entities_for_components(db, world, request, get_comp_from_db)?;

        let components = R::load_components_from_entities(world, entities);
        Ok(components)
    }

    fn create(
        db: &Self::Database,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        unsafe {
            let w = world.world_mut();
            w.spawn((
                component,
                DatabaseEntity {
                    id: db.get_key(),
                    persisted: false.into(),
                    dirty: false,
                    request,
                },
            ));
        }

        Ok(())
    }
}
