
use bevy_ecs::{component::Component, prelude::*};
use bevy_erm::{add_event, DatabaseEntity, DatabaseEntityIndex, DatabaseQuery, DatabaseQueryInfo, DatabaseResource, AnyDatabaseResource, ToBeCreatedDatabaseEntity};
use bevy_mod_index::prelude::*;
use futures::{executor::block_on, StreamExt};
use async_trait::async_trait;
use sqlx::{Encode, FromRow, Row};

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
    price: i32,
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
struct PurchasedItem {
    #[sqlx(flatten)]
    item: DatabaseEntity,
    #[sqlx(flatten)]
    buyer: DatabaseEntity,
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct TestComponent {
    age: u32,

}

struct ItemQuery {}
#[async_trait]
impl DatabaseQueryInfo for ItemQuery {
    type Component = MarketItem;
    type Database = AnyDatabaseResource;
    type Index = DatabaseEntityIndex;


    fn get_component<'c, E>(conn: E, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite> 
    { 
        let item = block_on(sqlx::query_as::<_, MarketItem>("SELECT item FROM items WHERE id = ?").bind(db_entity.id as i32).fetch_one(conn)).unwrap();
        Ok(item)
    }

    async fn write_component<'c, E>(tr: E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query("UPDATE items SET seller_id = ?, name = ?, price WHERE id = ?")
            .bind(component.seller_id.id as i64)
            .bind(component.name.clone())
            .bind(component.price)
            .bind(db_entity.id as i64)
            .execute(tr).await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn write_empty_entity<'c, E>(tr : E) -> Result<DatabaseEntity, ()> 
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite> {
        let r = sqlx::query("INSERT INTO items DEFAULT VALUES RETURNING *").execute(tr).await;

        
        match r {
            Ok(_) => {
                let id = 0; //todo get the id from the result
                Ok(DatabaseEntity{id})
            },
            Err(_) => Err(()),
        }
    }


}

struct PurchaseItemQuery {}
#[async_trait]
impl DatabaseQueryInfo for PurchaseItemQuery {
    type Component = PurchasedItem;
    type Database = AnyDatabaseResource;
    type Index = DatabaseEntityIndex;

    fn get_component<'c, E>(conn: E, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite> 
    {
        let item = block_on(sqlx::query_as::<_, PurchasedItem>("SELECT item FROM purchased_items WHERE id = ?").bind(db_entity.id).fetch_one(conn)).unwrap();
        Ok(item)
    }

    async fn write_component<'c, E>(tr: E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
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

    async fn write_empty_entity<'c, E>(tr : E) -> Result<DatabaseEntity, ()> 
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite> {
        let r = sqlx::query("INSERT INTO purchased_items DEFAULT VALUES RETURNING *").execute(tr).await;

        match r {
            Ok(_) => {
                let id = 0; //todo get the id from the result
                Ok(DatabaseEntity{id})
            },
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

fn purchase_system(mut events: EventReader<Purchase>, mut db_query: DatabaseQuery<ItemQuery>, mut db_query_purchased: DatabaseQuery<PurchaseItemQuery>) {
    for event in events.read() {
        let purchaser = db_query.get_mut(&event.purchaser).unwrap();
        let item = db_query.get(&event.item).unwrap();

        let purchased_item = PurchasedItem {
            item: event.item,
            buyer: event.purchaser,
        };

        db_query_purchased.create_entity(purchased_item);

    }
}

fn populate_db(db: ResMut<AnyDatabaseResource>) {
    let conn = db.get_connection();

    block_on(async {
        // create the tables
        // market items table
        sqlx::query("CREATE TABLE items (id INTEGER PRIMARY KEY, seller_id INTEGER, name TEXT, price INTEGER)")
            .execute(conn).await.unwrap();

        // purchased items table
        sqlx::query("CREATE TABLE purchased_items (id INTEGER PRIMARY KEY, item INTEGER, buyer INTEGER)")
            .execute(conn).await.unwrap();

        // users table
        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, buyer BOOLEAN, seller BOOLEAN)")
            .execute(conn).await.unwrap();

        // populate one buyer and one seller
        sqlx::query("INSERT INTO users (id, name, buyer, seller) VALUES (0, 'buyer', 1, 0)").execute(conn).await.unwrap();
        sqlx::query("INSERT INTO users (id, name, buyer, seller) VALUES (1, 'seller', 0, 1)").execute(conn).await.unwrap();

        // add one item to the market
        sqlx::query("INSERT INTO items (seller_id, name, price) VALUES (1, 'corn', 100)").execute(conn).await.unwrap();
    });
   
}

fn index_lookup(mut index: Index<DatabaseEntityIndex>, query: Query<&mut MarketItem>) {
    let entity_set = index.lookup(&0);
    let entity = *entity_set.iter().next().unwrap();
    let val = query.get(entity).unwrap();
    println!("index entity: {:?}", val);
}

fn flush_to_db(
    query: Query<(Entity, &DatabaseEntity, Option<&MarketItem>)>, 
    to_be_created_query : Query<(Entity), With<ToBeCreatedDatabaseEntity>>, 
    db_query : DatabaseQuery<ItemQuery>) {
    block_on(async {
        println!("flushing to db");

        let mut transaction = db_query.db.get_connection().begin().await.unwrap();

        for entity in to_be_created_query.iter() {
            db_query.write_empty_entity(&mut *transaction, &entity).await.unwrap();
        }

        println!("transaction started");
        for (_, db_entity, market_item) in query.iter() {
            if let Some(market_item) = market_item {
                db_query.write(&mut *transaction, &DatabaseEntity{id: db_entity.id}, &market_item).await.unwrap();
            }
        }

        transaction.commit().await.unwrap();


    });

    println!("flushed to db");

    block_on(async {
      // read the market_item database table
      let mut market_item = sqlx::query_as::<_, MarketItem>("SELECT * FROM items").bind(0).fetch(db_query.db.get_connection());
      market_item.for_each(|item| async {
          println!("item: {:?}", item.unwrap());
      }).await;
    });


}

#[derive(Event)]
struct TestEvent;

async fn run() {
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
        world.init_component::<PurchasedItem>();
    
        if !world.contains_resource::<AnyDatabaseResource>() {
            world.init_resource::<AnyDatabaseResource>();
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

#[tokio::main]
async fn main() {
    run().await;
}
