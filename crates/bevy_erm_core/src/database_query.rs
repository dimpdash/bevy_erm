use core::panic;

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

#[async_trait]
pub trait CustomDatabaseQuery<DbResource: DatabaseResource, DerefItem: Send>: Sync {
    async fn query(
        &self,
        tr: DatabaseTransaction<DbResource>,
    ) -> Result<Vec<(DatabaseEntity, DerefItem)>, ()>;
}

pub trait ReturnSelector<'w> {
    type ReturnItem;

    fn load_components_from_entities(
        world: UnsafeWorldCell<'w>,
        entities: Vec<Entity>,
    ) -> Vec<Self::ReturnItem>;
}

pub type DatabaseTransaction<'a, D> = <D as DatabaseResource>::Transaction;

// The items to be returned as readonly or mutable
// Use the World queries underlying specification
pub type ReadOnlyItem<'a, Q, DbResource> =
    ROQueryItem<'a, <Q as DBQueryInfo<DbResource>>::WorldQuery<'a>>;
pub type Item<'a, Q, DbResource> = QueryItem<'a, <Q as DBQueryInfo<DbResource>>::WorldQuery<'a>>;
#[async_trait]
pub trait DBQueryInfo<DbResource: DatabaseResource> {
    // type Mapper: ComponentMapper<Transaction = DbResource::Transaction>;
    // Using the world query to specify the type of the item
    // Allows for being able to use a bevy query from the DBQueryInfo
    // Used when inserting or updating components for the Database Query
    type WorldQuery<'a>: WorldQuery;
    type DerefItem: Send;
    type ReadOnlyItem<'a>: Send + From<ReadOnlyItem<'a, Self, DbResource>>;
    type Item<'a>: Send;

    async fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()>;
    async fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()>;
    async fn update_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()>;
    async fn insert_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()>;
    async fn load_components<'w, R: ReturnSelector<'w>, CDQ>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: CDQ,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>
    where
        CDQ: CustomDatabaseQuery<DbResource, Self::DerefItem> + Send;
    async fn create(
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

pub trait DatabaseEntityWithRequest: Sync + Send {
    fn request(&self) -> &RequestId;
    fn id(&self) -> &DatabaseEntityId;
}

impl<'w, 's, Q: DBQueryInfo<DbResource>, DbResource: DatabaseResource>
    DatabaseQuery<'w, 's, Q, DbResource>
{
    pub async fn get<D: DatabaseEntityWithRequest>(
        &self,
        db_entity: &D,
    ) -> Result<Q::ReadOnlyItem<'w>, ()> {
        Q::get(self.db.as_ref(), self.world, db_entity).await
    }

    pub async fn get_mut<D: DatabaseEntityWithRequest>(
        &self,
        db_entity: &D,
    ) -> Result<Q::Item<'w>, ()> {
        Q::get_mut(self.db.as_ref(), self.world, db_entity).await
    }

    pub async fn update_component(
        &self,
        db_entity: &DatabaseEntity,
        component: Q::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        Q::update_component(self.db.as_ref(), self.world, db_entity, component).await
    }

    pub async fn insert_component(
        &self,
        db_entity: &DatabaseEntity,
        component: Q::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        Q::insert_component(self.db.as_ref(), self.world, db_entity, component).await
    }

    pub async fn load_components<R: ReturnSelector<'w>, CDQ>(
        &self,
        request: RequestId,
        get_comp_from_db: CDQ,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>
    where
        CDQ: CustomDatabaseQuery<DbResource, Q::DerefItem> + Send,
    {
        Q::load_components::<R, CDQ>(self.db.as_ref(), self.world, request, get_comp_from_db).await
    }

    pub async fn create(&self, component: Q::DerefItem, request: RequestId) -> Result<(), ()> {
        Q::create(self.db.as_ref(), self.world, component, request).await
    }

    pub async fn update_or_insert_component(&self, entity: Entity) -> Result<(), ()> {
        unsafe {
            let mut q = self
                .world
                .world_mut()
                .query::<(&DatabaseEntity, Q::WorldQuery<'w>)>();
            let (db_entity, comp) = q.get(self.world.world(), entity).unwrap();

            if db_entity.persisted.into() {
                if db_entity.dirty {
                    Q::update_component(self.db.as_ref(), self.world, db_entity, comp.into()).await
                } else {
                    Ok(())
                }
            } else {
                Q::insert_component(self.db.as_ref(), self.world, db_entity, comp.into()).await
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
    type Executor: Send;

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

impl<T: DBQueryInfo<DbResource>, DbResource: DatabaseResource> TupleMarker<DbResource> for Option<T> where
    for<'a> Option<<T as DBQueryInfo<DbResource>>::ReadOnlyItem<'a>>:
        From<Option<ReadOnlyItem<'a, T, DbResource>>>
{
}

#[async_trait]
impl<T: DBQueryInfo<DbResource>, DbResource: DatabaseResource> DBQueryInfo<DbResource> for Option<T>
where
    for<'a> Option<<T as DBQueryInfo<DbResource>>::ReadOnlyItem<'a>>:
        From<Option<ReadOnlyItem<'a, T, DbResource>>>,
{
    type DerefItem = Option<<T as DBQueryInfo<DbResource>>::DerefItem>;

    // type Mapper = <T as DBQueryInfo<DbResource>>::Mapper;

    type WorldQuery<'a> = Option<<T as DBQueryInfo<DbResource>>::WorldQuery<'a>>;

    type ReadOnlyItem<'a> = Option<<T as DBQueryInfo<DbResource>>::ReadOnlyItem<'a>>;

    type Item<'a> = Option<<T as DBQueryInfo<DbResource>>::Item<'a>>;

    async fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()> {
        T::get(db, world, db_entity).await.map(Some)
    }

    async fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()> {
        T::get_mut(db, world, db_entity).await.map(Some)
    }

    async fn update_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        if let Some(component) = component {
            T::update_component(db, world, db_entity, component).await
        } else {
            Ok(())
        }
    }

    async fn insert_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        if let Some(component) = component {
            T::insert_component(db, world, db_entity, component).await
        } else {
            Ok(())
        }
    }

    async fn load_components<'w, R: ReturnSelector<'w>, CDQ>(
        _db: &DbResource,
        _world: UnsafeWorldCell<'w>,
        _request: RequestId,
        _get_comp_from_db: CDQ,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>
    where
        CDQ: CustomDatabaseQuery<DbResource, Self::DerefItem> + Send,
    {
        Err(())
    }

    async fn create(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        if let Some(component) = component {
            T::create(db, world, component, request).await
        } else {
            Ok(())
        }
    }
}

impl<'a, T: ComponentMapper, DbResource: DatabaseResource<Transaction = T::Executor>>
    TupleMarker<DbResource> for &T
where
    <T as ComponentMapper>::Component: Component,
{
}

impl<'a, T: ComponentMapper, DbResource: DatabaseResource<Transaction = T::Executor>>
    ReadMarker<DbResource> for &T
where
    <T as ComponentMapper>::Component: Component,
{
}

#[async_trait]
impl<'b, T: ComponentMapper, DbResource: DatabaseResource<Transaction = T::Executor>>
    DBQueryInfo<DbResource> for &T
where
    <T as ComponentMapper>::Component: Component,
{
    type DerefItem = <T as ComponentMapper>::Component;
    // type Mapper = T;
    type WorldQuery<'a> = &'a <T as ComponentMapper>::Component;

    type ReadOnlyItem<'a> = &'a <T as ComponentMapper>::Component;

    type Item<'a> = &'a <T as ComponentMapper>::Component;

    async fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()> {
        SingleComponentRetriever::<T, DbResource>::get(db, world, db_entity).await
    }

    async fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()> {
        SingleComponentRetriever::<T, DbResource>::get(db, world, db_entity).await
    }

    async fn update_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::update_component(db, world, db_entity, component)
            .await
    }

    async fn insert_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::insert_component(db, world, db_entity, component)
            .await
    }

    async fn load_components<'w, R: ReturnSelector<'w>, CDQ>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: CDQ,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>
    where
        CDQ: CustomDatabaseQuery<DbResource, Self::DerefItem> + Send,
    {
        SingleComponentRetriever::<T, DbResource>::load_components::<R, CDQ>(
            db,
            world,
            request,
            get_comp_from_db,
        )
        .await
    }

    async fn create(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::create(db, world, component, request).await
    }
}

impl<'b, T: ComponentMapper, DbResource: DatabaseResource<Transaction = T::Executor>>
    ReadMarker<DbResource> for &mut T
where
    <T as ComponentMapper>::Component: Component,
{
}

#[async_trait]
impl<'b, T: ComponentMapper, DbResource: DatabaseResource<Transaction = T::Executor>>
    DBQueryInfo<DbResource> for &mut T
where
    <T as ComponentMapper>::Component: Component,
{
    type DerefItem = <T as ComponentMapper>::Component;
    // type Mapper = T;
    type WorldQuery<'a> = &'a mut <T as ComponentMapper>::Component;

    type ReadOnlyItem<'a> = &'a <T as ComponentMapper>::Component;

    type Item<'a> = Mut<'a, <T as ComponentMapper>::Component>;

    async fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()> {
        SingleComponentRetriever::<T, DbResource>::get(db, world, db_entity).await
    }

    async fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()> {
        SingleComponentRetriever::<T, DbResource>::get_mut(db, world, db_entity).await
    }

    async fn update_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::update_component(db, world, db_entity, component)
            .await
    }

    async fn insert_component<'w>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: Self::ReadOnlyItem<'w>,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::insert_component(db, world, db_entity, component)
            .await
    }

    async fn load_components<'w, R: ReturnSelector<'w>, CDQ>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: CDQ,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>
    where
        CDQ: CustomDatabaseQuery<DbResource, Self::DerefItem> + Send,
    {
        SingleComponentRetriever::<T, DbResource>::load_components::<R, CDQ>(
            db,
            world,
            request,
            get_comp_from_db,
        )
        .await
    }

    async fn create(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        component: Self::DerefItem,
        request: RequestId,
    ) -> Result<(), ()> {
        SingleComponentRetriever::<T, DbResource>::create(db, world, component, request).await
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
        #[async_trait]
        impl<Z: TupleMarker<DbResource>, $($name: TupleMarker<DbResource>, )* DbResource: DatabaseResource> DBQueryInfo<DbResource> for (Z, $($name,)*)
            where
                for<'a>
                    (
                        <Z as DBQueryInfo<DbResource>>::ReadOnlyItem<'a>,
                        $(<$name as DBQueryInfo<DbResource>>::ReadOnlyItem<'a>, )*
                    )
                        :
                        From<(
                            ReadOnlyItem<'a, Z, DbResource>,
                            $(ReadOnlyItem<'a, $name, DbResource>,)*
                        )>
        {
            type DerefItem = (Z::DerefItem, $($name::DerefItem,)*);
            // type Mapper = NullMapper;
            type WorldQuery<'a> = (Z::WorldQuery<'a>, $($name::WorldQuery<'a>,)*);

            type ReadOnlyItem<'a> = (Z::ReadOnlyItem<'a>, $($name::ReadOnlyItem<'a>,)*);

            type Item<'a> = (Z::Item<'a>, $($name::Item<'a>,)*);

            async fn get<'w, DbEntity: DatabaseEntityWithRequest>(db: &DbResource, world: UnsafeWorldCell<'w>, db_entity: &DbEntity) -> Result<Self::ReadOnlyItem<'w>, ()> {
            //returns a tuple of all the gets
                Ok((
                    Z::get(db, world, db_entity).await?,
                    $(
                    {
                        $name::get(db, world, db_entity).await?
                    },
                )*))
            }

            async fn get_mut<'w, DbEntity: DatabaseEntityWithRequest>(db: &DbResource, world: UnsafeWorldCell<'w>, db_entity: &DbEntity) -> Result<Self::Item<'w>, ()> {
                //returns a tuple of all the gets
                Ok((
                    Z::get_mut(db, world, db_entity).await?,

                    $(
                    {
                        $name::get_mut(db, world, db_entity).await?
                    },
                )*))
            }

            async fn update_component<'w>(db: &DbResource, world: UnsafeWorldCell<'w>, db_entity: &DatabaseEntity, component: Self::ReadOnlyItem<'w>) -> Result<(), ()> {

                let (z, $(lower!($name),)*) = component;

                Z::update_component(db, world, db_entity, z).await?;
                $($name::update_component(db, world, db_entity, lower!($name)).await?;)*

                Ok(())
            }

            async fn insert_component<'w>(db: &DbResource, world: UnsafeWorldCell<'w>, db_entity: &DatabaseEntity, component: Self::ReadOnlyItem<'w>) -> Result<(), ()> {
                let (z, $(lower!($name),)*) = component;

                Z::insert_component(db, world, db_entity, z).await?;
                $($name::insert_component(db, world, db_entity, lower!($name)).await?;)*

                Ok(())
            }

            // CODE SMELL: probably should split up interface to avoid this method
            async fn load_components<'w, R : ReturnSelector<'w>, CDQ>(
                _db: &DbResource,
                _world: UnsafeWorldCell<'w>,
                _request: RequestId,
                _get_comp_from_db: CDQ
            ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>
            where
                CDQ: CustomDatabaseQuery<DbResource, Self::DerefItem> + Send
            {
                unimplemented!()
            }

            async fn create(db: &DbResource, world: UnsafeWorldCell<'_>, component: Self::DerefItem, request: RequestId) -> Result<(), ()> {
                let (z, $(lower!($name),)*) = component;

                Z::create(db, world, z, request).await?;
                $($name::create(db, world, lower!($name), request).await?;)*

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
        DbResource: DatabaseResource<Transaction = <MyMapper as ComponentMapper>::Executor>,
    > SingleComponentRetriever<MyMapper, DbResource>
where
    <MyMapper as ComponentMapper>::Component: Component,
{
    pub async fn get_internal<D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        db_entity: &D,
        component_preloaded: Option<<MyMapper as ComponentMapper>::Component>,
    ) -> Entity {
        let mut tr = db.get_transaction(*db_entity.request());

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
                            None => MyMapper::get(&mut tr, db_entity.id()).await.unwrap(),
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
                    None => MyMapper::get(&mut tr, db_entity.id()).await.unwrap(),
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

    pub async fn load_entities_for_components<CDQ>(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        request: RequestId,
        get_comp_from_db: CDQ,
    ) -> Result<Vec<Entity>, ()>
    where
        CDQ: CustomDatabaseQuery<DbResource, <MyMapper as ComponentMapper>::Component>,
    {
        let components = {
            let tr = db.get_transaction(request);

            get_comp_from_db.query(tr).await?
        };

        let mut entities = vec![];

        for (db_entity, component) in components {
            entities.push(Self::get_internal(db, world, &db_entity, Some(component)).await);
        }

        Ok(entities)
    }

    pub async fn update_or_insert_component(
        db: &DbResource,
        world: UnsafeWorldCell<'_>,
        db_entity: &DatabaseEntity,
        component: &MyMapper::Component,
    ) -> Result<(), ()> {
        if db_entity.persisted.into() {
            if db_entity.dirty {
                Self::update_component(db, world, db_entity, component).await
            } else {
                Ok(())
            }
        } else {
            Self::insert_component(db, world, db_entity, component).await
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

#[async_trait]
impl<
        'b,
        MyMapper: ComponentMapper,
        DbResource: DatabaseResource<Transaction = <MyMapper as ComponentMapper>::Executor>,
    > DBQueryInfo<DbResource> for SingleComponentRetriever<MyMapper, DbResource>
where
    <MyMapper as ComponentMapper>::Component: Component,
{
    type DerefItem = MyMapper::Component;
    type WorldQuery<'a> = &'a mut MyMapper::Component;
    type ReadOnlyItem<'a> = &'a MyMapper::Component;
    type Item<'a> = Mut<'a, MyMapper::Component>;

    async fn get<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::ReadOnlyItem<'w>, ()> {
        let entity = Self::get_internal(db, world, db_entity, None).await;

        unsafe {
            Ok(world
                .world()
                .get::<<MyMapper as ComponentMapper>::Component>(entity)
                .unwrap())
        }
    }

    async fn get_mut<'w, D: DatabaseEntityWithRequest>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        db_entity: &D,
    ) -> Result<Self::Item<'w>, ()> {
        let entity = Self::get_internal(db, world, db_entity, None).await;

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

    async fn update_component<'w>(
        db: &DbResource,
        _world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        let mut tr = db.get_transaction(*db_entity.request());

        MyMapper::update_component(&mut tr, db_entity.id(), component).await
    }

    async fn insert_component<'w>(
        db: &DbResource,
        _world: UnsafeWorldCell<'w>,
        db_entity: &DatabaseEntity,
        component: ReadOnlyItem<'w, Self, DbResource>,
    ) -> Result<(), ()> {
        let mut tr = db.get_transaction(*db_entity.request());
        MyMapper::insert_component(&mut tr, db_entity.id(), component).await
    }

    async fn load_components<'w, R: ReturnSelector<'w>, CDQ>(
        db: &DbResource,
        world: UnsafeWorldCell<'w>,
        request: RequestId,
        get_comp_from_db: CDQ,
    ) -> Result<Vec<<R as ReturnSelector<'w>>::ReturnItem>, ()>
    where
        CDQ: CustomDatabaseQuery<DbResource, Self::DerefItem> + Send,
    {
        let entities =
            Self::load_entities_for_components::<CDQ>(db, world, request, get_comp_from_db).await?;

        let components = R::load_components_from_entities(world, entities);
        Ok(components)
    }

    async fn create(
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
