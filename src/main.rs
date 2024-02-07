use bevy_ecs::{component::Component, prelude::*, schedule::LazyLoadedExecutor, system::SystemParam};
use bevy_reflect::{prelude::*};

use bevy_utils::petgraph::visit::Data;
use sqlx::{pool::PoolConnection, sqlite::*, Connection, Database, Row};
use std::{any::Any, borrow::BorrowMut, env};
use futures::executor::block_on;

#[derive(Event)]
struct PositionChanged{ entity: Entity, position: Position}


#[derive(Component)]
struct Position { x: f32, y: f32 }
#[derive(Component)]
struct Velocity { x: f32, y: f32 }

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
        let mut pos = query.get_mut(event.entity).unwrap();
        pos.x = event.position.x;
        pos.y = event.position.y;
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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Component)]
pub struct DatabaseEntity {
    id: u32
}

pub struct DatabaseEntities {
    db_entities: Vec<DatabaseEntity>
}

// stop type warning
type Pool<DbR> = sqlx::Pool<<DbR as DatabaseResource>::Database>;

pub trait DatabaseQueryInfo: Sized {
    type Component: Component + Reflect + Default;
    type Database: DatabaseResource;

    fn get_component(conn: &Pool<Self::Database>, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>;
    fn write_component(db_entity: &DatabaseEntity, component: Self::Component) -> Result<(), ()>;
}

pub struct DatabaseQueryFetchState<'w, 's, I: DatabaseQueryInfo + 'static> {
    db_state: <ResMut<'w, I::Database> as SystemParam>::State,
    phantom: std::marker::PhantomData<&'s ()>,
}

pub struct DatabaseQuery<'w, I:DatabaseQueryInfo + 'static> {
    db: ResMut<'w, I::Database>,
    world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>
}

// pub type RODatabaseQueryItem<'a, I> = &'a I::Component;

impl<'w, I:DatabaseQueryInfo> DatabaseQuery<'w, I> {
    pub fn get(&mut self, db_entity : &DatabaseEntity) -> Result<I::Component, ()> {
        let conn = self.db.get_connection();

        let comp = I::get_component(conn, db_entity);

        unsafe { 
            let w = self.world.world_mut(); 

            
            w.spawn((
                DatabaseEntity {id : db_entity.id},
                Age{ age : 15}
            ));
        }


        comp
    }

    pub fn write(&mut self, db_entity : &DatabaseEntity, component: I::Component) -> Result<(), ()> {
        I::write_component(db_entity, component)
    }
}

unsafe impl<'w, I:DatabaseQueryInfo> SystemParam for DatabaseQuery<'w, I>
    where I: DatabaseQueryInfo + 'static
{
    type State = DatabaseQueryFetchState<'static, 'static, I>;

    type Item<'_w, '_s> = DatabaseQuery<'_w, I>;

    fn init_state(world: &mut World, system_meta: &mut bevy_ecs::system::SystemMeta) -> Self::State {
        // https://github.com/chrisjuchem/bevy_mod_index/blob/15e9b4c9bbf26d4fc087ce056b07d1312464de2f/src/index.rs#L108
        if !world.contains_resource::<SqliteDatabaseResource>() {
            world.init_resource::<SqliteDatabaseResource>();
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
    ) -> Self::Item<'w2, 's2> 
    {
        let db_query = DatabaseQuery {
            db: <ResMut<'w2, I::Database>>::get_param(
                &mut state.db_state,
                system_meta,
                world,
                change_tick),
            world: world
        
        };

        db_query
    }
}

#[derive(Component, Reflect, Debug, Default)]
struct Age {
    age: u32
}

struct AgeQuery {}
impl DatabaseQueryInfo for AgeQuery {
    type Component = Age;
    type Database = SqliteDatabaseResource;

    fn get_component(conn: &sqlx::Pool<<Self::Database as DatabaseResource>::Database>, db_entity: &DatabaseEntity) -> Result<Age, ()> {
        let age = block_on(sqlx::query("SELECT age FROM person WHERE id = ?").bind(db_entity.id).fetch_one(conn)).unwrap();
        let age = age.get(0);
        Ok(Age {age: age})
    }

    fn write_component(_db_entity: &DatabaseEntity, _component: Age) -> Result<(), ()> {
        Ok(())
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
    let db_entity = DatabaseEntity {
        id: 0
    };
    let age = db_query.get(&db_entity).unwrap();
    let new_age = Age {
        age: age.age + 1
    };

    db_query.write(&db_entity, new_age).unwrap();
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
        let conn = block_on(self.pool.acquire()).unwrap();
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

#[tokio::main]
async fn main() {
    // Create a new empty World to hold our Entities and Components
    let mut world = World::new();

    // Spawn an entity with Position and Velocity components
    world.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 0.0 },
    ));

    world.init_component::<Age>();

    if !world.contains_resource::<SqliteDatabaseResource>() {
        world.init_resource::<SqliteDatabaseResource>();
    }


    // Create a new Schedule, which defines an execution strategy for Systems
    let executor = Box::new(LazyLoadedExecutor::new());
    let mut schedule = Schedule::new_with_executor(executor);

    add_event::<PositionChanged>(&mut world);

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
    startup_schedule.add_systems(populate_db.before(increment_age_system));
    startup_schedule.run(&mut world);

    schedule.add_systems(increment_age_system.before(lookup_db_query_system));
    schedule.add_systems(lookup_db_query_system);
    // schedule.add_systems(movement_changes);

    // Run the schedule once. If your app has a "loop", you would run this once per loop
    schedule.run(&mut world);

       let comps = world.components();

    println!("len: {:?}", comps.len());

    let age = Age {
        age: 15
    };

    // let type_id = age.type_id();
    let comp_id = comps.component_id::<Age>().unwrap();
    let name = comps.get_name(comp_id).unwrap();
    println!("name: {:?}", name);

}