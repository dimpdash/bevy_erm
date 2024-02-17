use crate::*;
use async_trait::async_trait;

use bevy_ecs::query::{QueryItem, ROQueryItem, WorldQuery};
use bevy_ecs::{
    component::Component, prelude::*, system::SystemParam,
    world::unsafe_world_cell::UnsafeWorldCell,
};
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use casey::lower;
use futures::executor::block_on;

pub trait ReturnSelector<'w> {
    type ReturnItem;

    fn load_components_from_entities(
        world: UnsafeWorldCell<'w>,
        entities: Vec<Entity>,
    ) -> Vec<Self::ReturnItem>;
}

pub type DatabaseTransaction<'a, 'b, D> = &'a mut <D as DatabaseResource>::Executor;

// The items to be returned as readonly or mutable
// Use the World queries underlying specification
pub type ReadOnlyItem<'a, Q, DbResource> =
    ROQueryItem<'a, <Q as DBQueryInfo<DbResource>>::WorldQuery<'a>>;
pub type Item<'a, Q, DbResource> = QueryItem<'a, <Q as DBQueryInfo<DbResource>>::WorldQuery<'a>>;

pub trait DBQueryInfo<DbResource: DatabaseResource> {
    // type Mapper: ComponentMapper<Executor = DbResource::Transaction>;
    // Using the world query to specify the type of the item
    // Allows for being able to use a bevy query from the DBQueryInfo
    // Used when inserting or updating components for the Database Query
    type WorldQuery<'a>: WorldQuery;
    type DerefItem;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<ReadOnlyItem<'w, Self, DbResource>, ()>;
    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Item<'w, Self, DbResource>, ()>;
    fn update_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()>;
    fn insert_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()>;
    fn load_components<'w, R: ReturnSelector<'w>>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseTransaction<DbResource>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>;
    fn create(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()>;
}

pub struct QueryFetchState<'w, 's, DbResource: DatabaseResource> {
    db_state: <Res<'w, DbResource> as SystemParam>::State,
    phantom: std::marker::PhantomData<&'s ()>,
}

pub struct DatabaseQuery<'world, 'state, Q: DBQueryInfo<DbResource>, DbResource: DatabaseResource> {
    // world and state will be needed later
    db: Res<'world, DbResource>,
    world: UnsafeWorldCell<'world>,
    phantom2: std::marker::PhantomData<(&'state (), Q)>,
}

pub trait B {}

pub trait A<B> {}

// So query can be constructed by the system
unsafe impl<'w, 's, I: DBQueryInfo<DbResource>, DbResource: DatabaseResource> SystemParam
    for DatabaseQuery<'w, 's, I, DbResource>
where
    I: DBQueryInfo<DbResource> + 'static,
{
    type State = QueryFetchState<'static, 'static, DbResource>;

    type Item<'world, 'state> = DatabaseQuery<'world, 'state, I, DbResource>;

    fn init_state(
        world: &mut World,
        system_meta: &mut bevy_ecs::system::SystemMeta,
    ) -> Self::State {
        // https://github.com/chrisjuchem/bevy_mod_index/blob/15e9b4c9bbf26d4fc087ce056b07d1312464de2f/src/index.rs#L108
        if !world.contains_resource::<DbResource>() {
            world.init_resource::<DbResource>();
        }

        QueryFetchState {
            db_state: <Res<'w, DbResource>>::init_state(world, system_meta),
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
            db: <Res<'w2, DbResource>>::get_param(
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

impl<'w, 's, Q: DBQueryInfo<DbResource>, DbResource: DatabaseResource>
    DatabaseQuery<'w, 's, Q, DbResource>
{
    pub fn get<D: DatabaseEntityWithRequest>(
        &self,
        db_entity: &D,
    ) -> Result<ReadOnlyItem<Q, DbResource>, ()> {
        Q::get(self.db.as_ref(), self.world, db_entity)
    }

    pub fn get_mut<D: DatabaseEntityWithRequest>(
        &self,
        db_entity: &D,
    ) -> Result<Item<Q, DbResource>, ()> {
        Q::get_mut(self.db.as_ref(), self.world, db_entity)
    }

    pub fn update_component(
        &self,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Q, DbResource>,
    ) -> Result<(), ()> {
        Q::update_component(self.db.as_ref(), self.world, db_entity, component)
    }

    pub fn insert_component(
        &self,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Q, DbResource>,
    ) -> Result<(), ()> {
        Q::insert_component(self.db.as_ref(), self.world, db_entity, component)
    }

    pub fn load_components<R: ReturnSelector<'w>>(
        &self,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseTransaction<DbResource>,
        ) -> Result<Vec<(DatabaseEntity, Q::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        Q::load_components::<R>(self.db.as_ref(), self.world, request, get_comp_from_db)
    }

    pub fn create(&self, component: Q::DerefItem, request: RequestId) -> Result<(), ()> {
        Q::create(self.db.as_ref(), self.world, component, request)
    }

    pub fn update_or_insert_component(&self, entity: Entity) -> Result<(), ()> {
        unsafe {
            let mut q = self
                .world
                .world_mut()
                .query::<(&DatabaseEntity, Q::WorldQuery<'w>)>();
            let (db_entity, comp) = q.get(self.world.world(), entity).unwrap();

            if db_entity.persisted.into() {
                if db_entity.dirty {
                    Q::update_component(self.db.as_ref(), self.world, db_entity, comp)
                } else {
                    Ok(())
                }
            } else {
                Q::insert_component(self.db.as_ref(), self.world, db_entity, comp)
            }
        }
    }

    pub fn commit(&self, request: RequestId) -> Result<(), ()> {
        self.db.commit_transaction(request);

        Ok(())
    }
}

#[async_trait]
pub trait ComponentMapper {
    type Component;
    type Executor;

    async fn get<'c>(
        e: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
    ) -> Result<Self::Component, ()>;

    async fn update_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()>;

    async fn insert_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()>;
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
    type Executor = ();

    async fn get<'c>(
        _e: &mut Self::Executor,
        _db_entity: &DatabaseEntityId,
    ) -> Result<Self::Component, ()> {
        unimplemented!()
    }

    async fn update_component<'c>(
        _tr: &mut Self::Executor,
        _db_entity: &DatabaseEntityId,
        _component: &Self::Component,
    ) -> Result<(), ()> {
        unimplemented!()
    }

    async fn insert_component<'c>(
        _tr: &mut Self::Executor,
        _db_entity: &DatabaseEntityId,
        _component: &Self::Component,
    ) -> Result<(), ()> {
        unimplemented!()
    }
}

// Used to help speicfy whether the returned component is read only or mutable
pub trait ReadMarker<DbResource: DatabaseResource>: DBQueryInfo<DbResource> {}

// traits that are expected to be used in a tuple
// &, &mut, Option
pub trait TupleMarker<DbResource: DatabaseResource>: DBQueryInfo<DbResource> {}

impl<T: DBQueryInfo<DbResource>, DbResource: DatabaseResource> TupleMarker<DbResource>
    for Option<T>
{
}

impl<T: DBQueryInfo<DbResource>, DbResource: DatabaseResource> DBQueryInfo<DbResource>
    for Option<T>
{
    type DerefItem = Option<<T as DBQueryInfo<DbResource>>::DerefItem>;

    // type Mapper = <T as DBQueryInfo<DbResource>>::Mapper;

    type WorldQuery<'a> = Option<<T as DBQueryInfo<DbResource>>::WorldQuery<'a>>;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<ReadOnlyItem<'w, Self, DbResource>, ()> {
        T::get(db, world, db_entity).map(Some)
    }

    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Item<'w, Self, DbResource>, ()> {
        T::get_mut(db, world, db_entity).map(Some)
    }

    fn update_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        if let Some(component) = component {
            T::update_component(db, world, db_entity, component)
        } else {
            Ok(())
        }
    }

    fn insert_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        if let Some(component) = component {
            T::insert_component(db, world, db_entity, component)
        } else {
            Ok(())
        }
    }

    fn load_components<'w, R: ReturnSelector<'w>>(
        _db: &DbResource,
        _world: UnsafeWorldCell<'w>,
        _request: RequestId,
        _get_comp_from_db: impl FnOnce(
            DatabaseTransaction<DbResource>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        unimplemented!()
    }

    fn create(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        if let Some(component) = component {
            T::create(db, world, component, request)
        } else {
            Ok(())
        }
    }
}

impl<'a, T: ComponentMapper, DbResource: DatabaseResource<Executor = T::Executor>>
    TupleMarker<DbResource> for &T
where
    <T as ComponentMapper>::Component: Component,
{
}

impl<'a, T: ComponentMapper, DbResource: DatabaseResource<Executor = T::Executor>>
    ReadMarker<DbResource> for &T
where
    <T as ComponentMapper>::Component: Component,
{
}

impl<'b, T: ComponentMapper, DbResource: DatabaseResource<Executor = T::Executor>>
    DBQueryInfo<DbResource> for &T
where
    <T as ComponentMapper>::Component: Component,
{
    type DerefItem = <T as ComponentMapper>::Component;
    // type Mapper = T;
    type WorldQuery<'a> = &'a <T as ComponentMapper>::Component;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<ReadOnlyItem<'w, Self, DbResource>, ()> {
        SingleComponentRetriever::<T, DbResource>::get(db, world, db_entity)
    }

    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Item<'w, Self, DbResource>, ()> {
        SingleComponentRetriever::<T, DbResource>::get(db, world, db_entity)
    }

    fn update_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::update_component(db, world, db_entity, component)
    }

    fn insert_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::insert_component(db, world, db_entity, component)
    }

    fn load_components<'w, R: ReturnSelector<'w>>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseTransaction<'_, 'b, DbResource>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        SingleComponentRetriever::<T, DbResource>::load_components::<R>(
            db,
            world,
            request,
            get_comp_from_db,
        )
    }

    fn create(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::create(db, world, component, request)
    }
}

impl<'b, T: ComponentMapper, DbResource: DatabaseResource<Executor = T::Executor>>
    ReadMarker<DbResource> for &mut T
where
    <T as ComponentMapper>::Component: Component,
{
}

impl<'b, T: ComponentMapper, DbResource: DatabaseResource<Executor = T::Executor>>
    DBQueryInfo<DbResource> for &mut T
where
    <T as ComponentMapper>::Component: Component,
{
    type DerefItem = <T as ComponentMapper>::Component;
    // type Mapper = T;
    type WorldQuery<'a> = &'a mut <T as ComponentMapper>::Component;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<ReadOnlyItem<'w, Self, DbResource>, ()> {
        SingleComponentRetriever::<T, DbResource>::get(db, world, db_entity)
    }

    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Item<'w, Self, DbResource>, ()> {
        SingleComponentRetriever::<T, DbResource>::get_mut(db, world, db_entity)
    }

    fn update_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::update_component(db, world, db_entity, component)
    }

    fn insert_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::insert_component(db, world, db_entity, component)
    }

    fn load_components<'w, R: ReturnSelector<'w>>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseTransaction<'_, 'b, DbResource>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        SingleComponentRetriever::<T, DbResource>::load_components::<R>(
            db,
            world,
            request,
            get_comp_from_db,
        )
    }

    fn create(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::create(db, world, component, request)
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
        impl<Z: TupleMarker<DbResource>, $($name: TupleMarker<DbResource>, )* DbResource: DatabaseResource> DBQueryInfo<DbResource> for (Z, $($name,)*)
        {
            type DerefItem = (Z::DerefItem, $($name::DerefItem,)*);
            // type Mapper = NullMapper;
            type WorldQuery<'a> = (Z::WorldQuery<'a>, $($name::WorldQuery<'a>,)*);

            fn get<'w, DbEntity: DatabaseEntityWithRequest>(db: &DbResource, world: UnsafeWorldCell<'w>, db_entity: &DbEntity) -> Result<ReadOnlyItem<'w, Self, DbResource>, ()> {
            //returns a tuple of all the gets
                Ok((
                    Z::get(db, world, db_entity)?,
                    $(
                    {
                        $name::get(db, world, db_entity)?
                    },
                )*))
            }

            fn get_mut<'w, DbEntity: DatabaseEntityWithRequest>(db: &DbResource, world: UnsafeWorldCell<'w>, db_entity: &DbEntity) -> Result<Item<'w, Self, DbResource>, ()> {
                //returns a tuple of all the gets
                Ok((
                    Z::get_mut(db, world, db_entity)?,

                    $(
                    {
                        $name::get_mut(db, world, db_entity)?
                    },
                )*))
            }

            fn update_component<'w>(db: &DbResource, world: UnsafeWorldCell<'w>, db_entity: &DatabaseEntity, component: ReadOnlyItem<'w, Self, DbResource>) -> Result<(), ()> {

                let (z, $(lower!($name),)*) = component;

                Z::update_component(db, world, db_entity, z)?;
                $($name::update_component(db, world, db_entity, lower!($name))?;)*

                Ok(())
            }

            fn insert_component<'w>(db: &DbResource, world: UnsafeWorldCell<'w>, db_entity: &DatabaseEntity, component: ReadOnlyItem<'w, Self, DbResource>) -> Result<(), ()> {
                let (z, $(lower!($name),)*) = component;

                Z::insert_component(db, world, db_entity, z)?;
                $($name::insert_component(db, world, db_entity, lower!($name))?;)*

                Ok(())
            }

            // CODE SMELL: probably should split up interface to avoid this method
            fn load_components<'w, R : ReturnSelector<'w>>(
                _db: &DbResource,
                _world: UnsafeWorldCell<'w>,
                _request: RequestId,
                _get_comp_from_db: impl FnOnce(DatabaseTransaction<DbResource>) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>
            ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
                unimplemented!()
            }

            fn create(db: &DbResource, world: UnsafeWorldCell<'_>, component: Self::DerefItem, request: RequestId) -> Result<(), ()> {
                let (z, $(lower!($name),)*) = component;

                Z::create(db, world, z, request)?;
                $($name::create(db, world, lower!($name), request)?;)*

                Ok(())
            }
        }
    }
}

// Create a simple composition of DBQueryInfo for tuples of length 1 to 10
// Allows DBQueryInfo to be composed of other DBQueryInfo
// eg. DBQuery<(User, Item)>
// simple_composition_of_db_queries!{}
simple_composition_of_db_queries! {A}
simple_composition_of_db_queries! {A B}
simple_composition_of_db_queries! {A B C}
simple_composition_of_db_queries! {A B C D}
// simple_composition_of_db_queries!{A B C D E}
// simple_composition_of_db_queries!{A B C D E F}
// simple_composition_of_db_queries!{A B C D E F G}
// simple_composition_of_db_queries!{A B C D E F G H}
// simple_composition_of_db_queries!{A B C D E F G H I}
// simple_composition_of_db_queries!{A B C D E F G H I J}
// simple_composition_of_db_queries!{A B C D E F G H I J K}

#[derive(Default)]
pub struct SingleComponentRetriever<Mapper, DbResource> {
    phantom: std::marker::PhantomData<(Mapper, DbResource)>,
}

impl<
        'a,
        MyMapper: ComponentMapper,
        DbResource: DatabaseResource<Executor = <MyMapper as ComponentMapper>::Executor>,
    > SingleComponentRetriever<MyMapper, DbResource>
where
    <MyMapper as ComponentMapper>::Component: Component,
{
    fn get_internal<D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        db_entity: &D,
        component_preloaded: Option<<MyMapper as ComponentMapper>::Component>,
    ) -> Entity {
        let arc = db.get_transaction(*db_entity.request());
        let mut a = arc.write().unwrap();
        let tr = a.as_mut().unwrap();

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
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseTransaction<'_, 'a, DbResource>,
        ) -> Result<
            Vec<(DatabaseEntity, <MyMapper as ComponentMapper>::Component)>,
            (),
        >,
    ) -> Result<Vec<Entity>, ()> {
        let components = {
            let arc = db.get_transaction(request);
            let mut a = arc.write().unwrap();
            let tr = a.as_mut().unwrap();

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

    pub fn update_or_insert_component(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        db_entity: &DatabaseEntity,
        component: &MyMapper::Component,
    ) -> Result<(), ()> {
        if db_entity.persisted.into() {
            if db_entity.dirty {
                Self::update_component(db, world, db_entity, component)
            } else {
                Ok(())
            }
        } else {
            Self::insert_component(db, world, db_entity, component)
        }
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

impl<
        'b,
        MyMapper: ComponentMapper,
        DbResource: DatabaseResource<Executor = <MyMapper as ComponentMapper>::Executor>,
    > DBQueryInfo<DbResource> for SingleComponentRetriever<MyMapper, DbResource>
where
    <MyMapper as ComponentMapper>::Component: Component,
{
    type DerefItem = MyMapper::Component;
    type WorldQuery<'a> = &'a mut MyMapper::Component;

    fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<ReadOnlyItem<'w, Self, DbResource>, ()> {
        let entity = Self::get_internal(db, world, db_entity, None);

        unsafe {
            Ok(world
                .world()
                .get::<<MyMapper as ComponentMapper>::Component>(entity)
                .unwrap())
        }
    }

    fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Item<'w, Self, DbResource>, ()> {
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
        db: &DbResource,
        _world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        let arc = db.get_transaction(*db_entity.request());
        let mut a = arc.write().unwrap();
        let tr = a.as_mut().unwrap();

        block_on(MyMapper::update_component(
            &mut **tr,
            db_entity.id(),
            component,
        ))
    }

    fn insert_component<'w>(
        db: &DbResource,
        _world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        let arc = db.get_transaction(*db_entity.request());
        let mut a = arc.write().unwrap();
        let tr = a.as_mut().unwrap();

        block_on(MyMapper::insert_component(
            &mut **tr,
            db_entity.id(),
            component,
        ))
    }

    fn load_components<'w, R: ReturnSelector<'w>>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: impl FnOnce(
            DatabaseTransaction<'_, 'b, DbResource>,
        ) -> Result<Vec<(DatabaseEntity, Self::DerefItem)>, ()>,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()> {
        let entities = Self::load_entities_for_components(db, world, request, get_comp_from_db)?;

        let components = R::load_components_from_entities(world, entities);
        Ok(components)
    }

    fn create(
        db: &DbResource,
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
