use std::{any::TypeId, ops::Deref};

use bevy_ecs::{component::Component, prelude::*, system::SystemParam};
use bevy_reflect::prelude::*;
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use sqlx::{database::HasArguments, sqlite::*, Row, Transaction};
use futures::{executor::block_on, Future, StreamExt};
use bevy_mod_index::index::IndexFetchState;
use async_trait::async_trait;

#[derive(Event, Debug)]
struct PositionChanged{ entity: Entity, position: Position}


#[derive(Component, Debug)]
struct Position { x: f32, y: f32 }
#[derive(Component)]
struct Velocity { x: f32, y: f32 }

struct DatabaseEntityIndex;
impl IndexInfo for DatabaseEntityIndex {
    type Component = DatabaseEntity;

    type Value = u32;

    type Storage = NoStorage<Self>;


    fn value(c: &Self::Component) -> Self::Value {
        c.id
    }
}

// This system moves each entity with a Position and Velocity component
fn movement(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut position, velocity) in &mut query {
        position.x += velocity.x;
        position.y += velocity.y;
    }
}

fn gen_move_updates(mut query: Query<(Entity, &Position, &Velocity)>, mut events: EventWriter<PositionChanged>) {
    println!("gen_move_updates");
    for (entity, position, velocity) in &mut query {
        events.send(PositionChanged {
            entity: entity,
            position: Position {
                x: position.x + velocity.x,
                y: position.y + velocity.y,
            },
        });
    }
}


fn movement_changes(mut query: Query<&mut Position>, mut events: EventReader<PositionChanged>) {
    println!("movement_changes");
    for event in events.read() {
        println!("event: {:?}", event);
    }
}

fn print_position(query: Query<&Position>) {
    for position in &query {
        println!("Position: ({}, {})", position.x, position.y);
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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Component, Debug)]
pub struct DatabaseEntity {
    id: u32
}

pub struct DatabaseEntities {
    db_entities: Vec<DatabaseEntity>
}

// stop type warning
type Pool<DbR> = sqlx::Pool<<DbR as DatabaseResource>::Database>;

type SqlxQueryAlias<'a, 'b, DBR> = sqlx::query::Query<'a, <DBR as DatabaseResource>::Database, <<DBR as DatabaseResource>::Database as HasArguments<'b>>::Arguments>;

#[async_trait]
pub trait DatabaseQueryInfo: Sized {
    type Component: Component + Reflect + Default;
    type Database: DatabaseResource;
    type Index: IndexInfo;

    fn get_component(conn: &Pool<Self::Database>, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>;
    async fn write_component<'c, E>(tr : E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()> 
    where
        E: sqlx::Executor<'c, Database = <Self::Database as DatabaseResource>::Database>,;
}

pub struct DatabaseQueryFetchState<'w, 's, I: DatabaseQueryInfo + 'static> {
    db_state: <ResMut<'w, I::Database> as SystemParam>::State,
    phantom: std::marker::PhantomData<&'s ()>,
}

pub struct DatabaseQuery<'w, 's, I:DatabaseQueryInfo + 'static> {
    db: ResMut<'w, I::Database>,
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

#[derive(Component, Reflect, Debug, Default, Clone, sqlx::FromRow)]
struct Age {
    age: u32
}


struct AgeQuery {}
#[async_trait]
impl DatabaseQueryInfo for AgeQuery {
    type Component = Age;
    type Database = SqliteDatabaseResource;
    type Index = DatabaseEntityIndex;


    fn get_component(conn: &Pool<Self::Database>, db_entity: &DatabaseEntity) -> Result<Age, ()> {
        let age = block_on(sqlx::query("SELECT age FROM person WHERE id = ?").bind(db_entity.id).fetch_one(conn)).unwrap();
        let age = age.get(0);
        Ok(Age {age: age})
    }

    async fn write_component<'c, E>(tr: E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = <Self::Database as DatabaseResource>::Database>,
    {
        let r = sqlx::query("UPDATE person SET age = ? WHERE id = ?")
            .bind(component.age)
            .bind(db_entity.id)
            .execute(tr).await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }


}

fn lookup_db_query_system(mut db_query: DatabaseQuery<AgeQuery>) {
    let db_entity = DatabaseEntity {
        id: 0
    };
    let age = db_query.get(&db_entity).unwrap();
    println!("age: {:?}", age);
}

fn increment_age_system(mut db_query: DatabaseQuery<AgeQuery>) {
    println!("incrementing age");
    let db_entity = DatabaseEntity {
        id: 0
    };
    let mut age = db_query.get_mut(&db_entity).unwrap();
    age.age += 1;
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

fn populate_db(db: ResMut<SqliteDatabaseResource>) {
    let conn = db.get_connection();
    block_on(sqlx::query("CREATE TABLE person (id INTEGER PRIMARY KEY, age INTEGER)")
    .execute(conn)).unwrap();
    block_on(sqlx::query("INSERT INTO person (id, age) VALUES (?, ?)")
        .bind(0)
        .bind(15)
        .execute(conn)).unwrap();
}

fn index_lookup(mut index: Index<DatabaseEntityIndex>, mut query: Query<(&mut Age)>) {
    let entity_set = index.lookup(&0);
    let entity = *entity_set.iter().next().unwrap();
    let val = query.get(entity).unwrap();
    println!("index entity: {:?}", val);
}

fn events_ended(mut events: EventReader<PositionChanged>) {
    events.is_empty();

}

fn clear_events(mut events: EventReader<PositionChanged>) {
    println!("clearing events");
    events.clear();
}

fn do_nothing() {
    println!("do nothing");
}

fn flush_to_db(query: Query<(Entity, &DatabaseEntity, Option<&Age>), With<DatabaseEntity>>, db_query : DatabaseQuery<AgeQuery>) {
    block_on(async {
        println!("flushing to db");

        let mut transaction = db_query.db.get_connection().begin().await.unwrap();

        println!("transaction started");
        for (_, db_entity, age) in query.iter() {
            if let Some(age) = age {
                db_query.write(&mut *transaction, &DatabaseEntity{id: db_entity.id}, &age).await.unwrap();
            }
        }

        transaction.commit().await.unwrap();


    });

    println!("flushed to db");

    block_on(async {
      // read the age database table
      let age = sqlx::query_as::<_, Age>("SELECT age FROM person").bind(0).fetch(db_query.db.get_connection());
      age.for_each(|age| async {
          println!("age: {:?}", age.unwrap());
      }).await;
      
    });


}

#[derive(Event)]
struct TestEvent;

#[tokio::main]
async fn main() {
    // Create a new empty World to hold our Entities and Components
    let mut world = World::new();

    // Spawn an entity with Position and Velocity components
    world.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 0.0 },
    ));

    world.spawn((
        DatabaseEntity {id: 1},
    ));

    world.init_component::<Age>();

    if !world.contains_resource::<SqliteDatabaseResource>() {
        world.init_resource::<SqliteDatabaseResource>();
    }


    // Create a new Schedule, which defines an execution strategy for Systems
    let mut schedule = Schedule::default();

    // add the events
    let mut clear_events_schedule = Schedule::default();
    add_event::<PositionChanged>(&mut world);
    clear_events_schedule.add_systems(bevy_ecs::event::event_update_system::<PositionChanged>);

    let mut query = world.query::<Entity>();

    let mut entities = vec![];
    for entity in query.iter(&world) {
        entities.push(entity);
    }

    let mut e = world.get_resource_mut::<Events<PositionChanged>>().unwrap();

    for entity in entities {
        e.send(PositionChanged {
            entity: entity,
            position: Position { x: 1.0, y: 1.0 },
        });
    }

 

    // want to find out for a particular component what its component id is 


    // Add our system to the schedule
    // schedule.add_systems(movement);
    let mut startup_schedule = Schedule::default();
    startup_schedule.add_systems(populate_db);
    startup_schedule.run(&mut world);

    schedule.add_systems(increment_age_system.before(lookup_db_query_system));
    schedule.add_systems(lookup_db_query_system);
    schedule.add_systems(index_lookup.after(increment_age_system));
    schedule.add_systems(movement_changes);
    // schedule.add_systems(do_nothing);


    let mut reader = IntoSystem::into_system(|mut events: EventReader<PositionChanged>| -> bool {
        println!("event count {}", events.len());
        !events.is_empty()
    });

    reader.initialize(&mut world);

    let mut still_events_to_read = |  world : &mut World | -> bool {reader.run((), world)};




    let mut count = 0;
    const MAX_COUNT : u32 = 3;

    // loop until all events are empty
    while still_events_to_read(&mut world) && count < MAX_COUNT {
        println!("running");
        schedule.run(&mut world);
        println!("ran");

        // clear all the events as they should have been read by all the systems
        clear_events_schedule.run(&mut world);
        count += 1;
    }   

    let mut flush_to_db_schedule = Schedule::default();
    flush_to_db_schedule.add_systems(flush_to_db);
    flush_to_db_schedule.run(&mut world);


    println!("done"); 

}
