use std::{any::TypeId, ops::Deref};

use bevy_ecs::{component::Component, prelude::*, system::SystemParam};
use bevy_erm::{add_event, DatabaseEntity, DatabaseEntityIndex, DatabaseQuery, DatabaseQueryInfo, DatabaseResource, Pool, SqliteDatabaseResource};
use bevy_reflect::prelude::*;
use bevy_mod_index::prelude::*;
use bevy_utils::hashbrown::HashSet;
use sqlx::{database::HasArguments, sqlite::*, Row, Transaction};
use futures::{executor::block_on, Future, StreamExt};
use bevy_mod_index::index::IndexFetchState;
use async_trait::async_trait;

#[derive(Event)]
pub struct Purchase {
    pub item : DatabaseEntity,
    pub purchaser : DatabaseEntity,
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct MarketItem {
    #[sqlx(flatten)]
    seller_id: DatabaseEntity,
    name: String,
    price: u32,
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct User {
    name: String,
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct Seller {}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct Buyer {}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct PurchasedItems {
    #[sqlx(flatten)]
    item: DatabaseEntity,
    #[sqlx(flatten)]
    buyer: DatabaseEntity,
}

struct ItemQuery {}
#[async_trait]
impl DatabaseQueryInfo for ItemQuery {
    type Component = MarketItem;
    type Database = SqliteDatabaseResource;
    type Index = DatabaseEntityIndex;


    fn get_component(conn: &Pool<Self::Database>, db_entity: &DatabaseEntity) -> Result<Self::Component, ()> {
        let item = block_on(sqlx::query_as::<_, MarketItem>("SELECT item FROM items WHERE id = ?").bind(db_entity.id).fetch_one(conn)).unwrap();
        Ok(item)
    }

    async fn write_component<'c, E>(tr: E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = <Self::Database as DatabaseResource>::Database>,
    {
        let r = sqlx::query("UPDATE items SET seller_id = ?, name = ?, price WHERE id = ?")
            .bind(component.seller_id.id)
            .bind(component.name.clone())
            .bind(component.price)
            .bind(db_entity.id)
            .execute(tr).await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }


}

struct PurchaseItemQuery {}
#[async_trait]
impl DatabaseQueryInfo for PurchaseItemQuery {
    type Component = PurchasedItems;
    type Database = SqliteDatabaseResource;
    type Index = DatabaseEntityIndex;

    fn get_component(conn: &Pool<Self::Database>, db_entity: &DatabaseEntity) -> Result<Self::Component, ()> {
        let item = block_on(sqlx::query_as::<_, PurchasedItems>("SELECT item FROM purchased_items WHERE id = ?").bind(db_entity.id).fetch_one(conn)).unwrap();
        Ok(item)
    }

    async fn write_component<'c, E>(tr: E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = <Self::Database as DatabaseResource>::Database>,
    {
        let r = sqlx::query("UPDATE purchased_items SET item = ?, buyer = ? WHERE id = ?")
            .bind(component.item.id)
            .bind(component.buyer.id)
            .bind(db_entity.id)
            .execute(tr).await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

fn lookup_db_query_system(mut db_query: DatabaseQuery<ItemQuery>) {
    let db_entity = DatabaseEntity {
        id: 0
    };
    let age = db_query.get(&db_entity).unwrap();
    println!("age: {:?}", age);
}

fn purchase_system(mut events: EventReader<Purchase>, mut db_query: DatabaseQuery<ItemQuery>) {
    for event in events.read() {
        let purchaser = db_query.get_mut(&event.purchaser).unwrap();
        let item = db_query.get(&event.item).unwrap();


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

fn index_lookup(mut index: Index<DatabaseEntityIndex>, query: Query<&mut MarketItem>) {
    let entity_set = index.lookup(&0);
    let entity = *entity_set.iter().next().unwrap();
    let val = query.get(entity).unwrap();
    println!("index entity: {:?}", val);
}

fn flush_to_db(query: Query<(Entity, &DatabaseEntity, Option<&MarketItem>), With<DatabaseEntity>>, db_query : DatabaseQuery<ItemQuery>) {
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
      let age = sqlx::query_as::<_, MarketItem>("SELECT * FROM items").bind(0).fetch(db_query.db.get_connection());
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

    let purchaser = DatabaseEntity {id: 0};

    // Spawn an entity
    world.spawn((
        purchaser,
    ));

    world.init_component::<MarketItem>();
    world.init_component::<User>();
    world.init_component::<Seller>();
    world.init_component::<Buyer>();
    world.init_component::<PurchasedItems>();

    if !world.contains_resource::<SqliteDatabaseResource>() {
        world.init_resource::<SqliteDatabaseResource>();
    }


    // Create a new Schedule, which defines an execution strategy for Systems
    let mut schedule = Schedule::default();

    // add the events
    let mut clear_events_schedule = Schedule::default();
    add_event::<Purchase>(&mut world);
    clear_events_schedule.add_systems(bevy_ecs::event::event_update_system::<Purchase>);

    let mut query = world.query::<Entity>();

    let mut entities = vec![];
    for entity in query.iter(&world) {
        entities.push(entity);
    }

    let mut e = world.get_resource_mut::<Events<Purchase>>().unwrap();

    // for entity in entities {
    //     e.send(Purchase {
    //         purchaser: purchaser,
    //         age: 0,
    //     });
    // }

 

    // want to find out for a particular component what its component id is 


    // Add our system to the schedule
    // schedule.add_systems(movement);
    let mut startup_schedule = Schedule::default();
    startup_schedule.add_systems(populate_db);
    startup_schedule.run(&mut world);

    // schedule.add_systems(increment_age_system.before(lookup_db_query_system));
    // schedule.add_systems(lookup_db_query_system);
    // schedule.add_systems(index_lookup.after(increment_age_system));
    // schedule.add_systems(do_nothing);


    let mut reader = IntoSystem::into_system(|events: EventReader<Purchase>| -> bool {
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
