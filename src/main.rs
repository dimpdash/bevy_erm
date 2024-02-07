use bevy_ecs::{prelude::*, schedule::LazyLoadedExecutor, system::SystemParam, component::Component};
use bevy_reflect::{prelude::*, DynamicStruct};

#[derive(Component)]
struct Position { x: f32, y: f32 }
#[derive(Component)]
struct Velocity { x: f32, y: f32 }

#[derive(Event)]
struct PositionChanged{ entity: Entity, position: Position}


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

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct DatabaseEntity {
    id: u32
}

pub struct DatabaseEntities {
    db_entities: Vec<DatabaseEntity>
}

pub trait DatabaseQueryInfo: Sized {
    type Component: Component + Reflect + Default;

    fn get_component(db_entity: &DatabaseEntity) -> Result<Self::Component, ()>;
    fn write_component(db_entity: &DatabaseEntity, component: Self::Component) -> Result<(), ()>;
}

pub struct DatabaseQueryFetchState {}

pub struct DatabaseQuery<I:DatabaseQueryInfo> {
    phantom: std::marker::PhantomData<I>
}

// pub type RODatabaseQueryItem<'a, I> = &'a I::Component;

impl<I:DatabaseQueryInfo> DatabaseQuery<I> {
    pub fn get(&mut self, db_entity : &DatabaseEntity) -> Result<I::Component, ()> {
        I::get_component(db_entity)
    }

    pub fn write(&mut self, db_entity : &DatabaseEntity, component: I::Component) -> Result<(), ()> {
        I::write_component(db_entity, component)
    }
}

unsafe impl<I:DatabaseQueryInfo> SystemParam for DatabaseQuery<I> {
    type State = DatabaseQueryFetchState;

    type Item<'world, 'state> = DatabaseQuery<I>;

    fn init_state(world: &mut World, system_meta: &mut bevy_ecs::system::SystemMeta) -> Self::State {
        //todo add cache ability by implementing storage
        // for now just keep refetching data
        // this is non-opitional as refectching existing data  thats changed locally would causes inconsistencies
        // use as example
        // https://github.com/chrisjuchem/bevy_mod_index/blob/15e9b4c9bbf26d4fc087ce056b07d1312464de2f/src/index.rs#L108

        // todo connect to db

        DatabaseQueryFetchState {}
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &bevy_ecs::system::SystemMeta,
        world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: bevy_ecs::component::Tick,
    ) -> Self::Item<'world, 'state> {
        let db_query = DatabaseQuery {phantom: std::marker::PhantomData};

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

    fn get_component(db_entity: &DatabaseEntity) -> Result<Age, ()> {
        Ok(Age {age: 5})
    }

    fn write_component(db_entity: &DatabaseEntity, component: Age) -> Result<(), ()> {
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


fn main() {
    // Create a new empty World to hold our Entities and Components
    let mut world = World::new();

    // Spawn an entity with Position and Velocity components
    world.spawn((
        Position { x: 0.0, y: 0.0 },
        Velocity { x: 1.0, y: 0.0 },
    ));


    // Create a new Schedule, which defines an execution strategy for Systems
    let executor = Box::new(LazyLoadedExecutor::new());
    let mut schedule = Schedule::new_with_executor(executor);

    add_event::<PositionChanged>(&mut world);

    let mut query = world.query::<(Entity)>();

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

    // Add our system to the schedule
    // schedule.add_systems(movement);
    schedule.add_systems(increment_age_system.before(lookup_db_query_system));
    schedule.add_systems(lookup_db_query_system);
    // schedule.add_systems(movement_changes);

    // Run the schedule once. If your app has a "loop", you would run this once per loop
    schedule.run(&mut world);

}