
use bevy_ecs::{component::Component, prelude::*};
use bevy_erm::{add_event, flush_component_to_db, AnyDatabaseResource, DatabaseEntity, DatabaseEntityIndex, DatabaseQuery, DatabaseQueryInfo, DatabaseResource, Persisted};
use bevy_mod_index::prelude::*;
use futures::{executor::block_on, stream::BoxStream, StreamExt};
use async_trait::async_trait;
use sqlx::{Encode, FromRow, Row};

#[derive(Event)]
pub struct Purchase {
    pub item : DatabaseEntity,
    pub purchaser : DatabaseEntity,
}

#[derive(Component, Debug, Default, Clone)]
struct MarketItem {
    seller_id: DatabaseEntity,
    name: String,
    price: i32,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for MarketItem {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(MarketItem {
            seller_id: DatabaseEntity { id: row.try_get("seller_id")?, persisted: true.into()},
            name: row.get("name"),
            price: row.get("price"),
        })
    }
}


#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct User {
    name: String,
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct Seller {}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct Buyer {}

#[derive(Component, Debug, Default, Clone)]
struct PurchasedItem {
    item: DatabaseEntity,
    buyer: DatabaseEntity,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for PurchasedItem {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(PurchasedItem {
            item: DatabaseEntity { id: row.try_get("item")?, persisted: true.into()},
            buyer: DatabaseEntity { id: row.try_get("buyer")?, persisted: true.into()},
        })
    }
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

    async fn update_component<'c, E>(tr: E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()>
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

    async fn insert_component<'c, E>(tr : E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()> 
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite> 
    {
        let r = sqlx::query("INSERT INTO items (id, seller_id, name, price) VALUES (?, ?, ?, ?)")
            .bind(db_entity.id)
            .bind(component.seller_id.id as i64)
            .bind(component.name.clone())
            .bind(component.price)
            .execute(tr).await;

            match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    fn table_name() -> Result<&'static str, ()> {
        Ok("items")
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

    async fn update_component<'c, E>(tr: E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()>
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

    async fn insert_component<'c, E>(tr : E, db_entity: &DatabaseEntity, component: &Self::Component) -> Result<(), ()> 
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite> 
    {
        let r = sqlx::query("INSERT INTO purchased_items (id, item, buyer) VALUES (?, ?, ?)")
            .bind(db_entity.id)
            .bind(component.item.id)
            .bind(component.buyer.id)
            .execute(tr).await;

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }


    fn table_name() -> Result<&'static str, ()> {
        Ok("purchased_items")
    }  

}   

fn purchase_system(mut events: EventReader<Purchase>, mut db_query_purchased: DatabaseQuery<PurchaseItemQuery>) {
    println!("purchase system");
    for event in events.read() {
        println!("purchased item: {:?}", event.item);
        let purchased_item = PurchasedItem {
            item: event.item,
            buyer: event.purchaser,
        };

        db_query_purchased.create_entity(purchased_item);

    }
}

fn populate_db(db: ResMut<AnyDatabaseResource>) {
    let db_handle = db.get_connection();
    let conn =&(*db_handle).write().unwrap().pool;

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
    });
   
}


async fn run() {
        // Create a new empty World to hold our Entities and Components
        let mut world = World::new();

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
    
        // Add our system to the schedule
        // schedule.add_systems(movement);
        let mut startup_schedule = Schedule::default();
        startup_schedule.add_systems(populate_db);
        startup_schedule.run(&mut world);

        // Fill the db with some data
        {
            let mut reader = IntoSystem::into_system(|db : ResMut<AnyDatabaseResource>, mut purchase_events : EventWriter<Purchase> |{
                let purchaser = 1;
                let seller = 2;
                let item = 3;
    
                block_on( async {
                    let db_handle = db.get_connection();
                    let conn =&(*db_handle).write().unwrap().pool;

                    // populate one buyer and one seller
                    sqlx::query("INSERT INTO users (id, name, buyer, seller) VALUES (?, 'buyer', 1, 0)").bind(purchaser).execute(conn).await.unwrap();
                    sqlx::query("INSERT INTO users (id, name, buyer, seller) VALUES (?, 'seller', 0, 1)").bind(seller).execute(conn).await.unwrap();
    
                    // add one item to the market
                    sqlx::query("INSERT INTO items (seller_id, name, price) VALUES (?, 'corn', 100)").bind(item).execute(conn).await.unwrap();
                });
    
                // add the triggering purchase event
    
                purchase_events.send(Purchase {
                    purchaser: DatabaseEntity {
                        id: purchaser,
                        persisted: true.into(),
                    },
                    item: DatabaseEntity {
                        id: item,
                        persisted: true.into(),
                    },
                });
            });

            reader.initialize(&mut world);
            reader.run((), &mut world);
        }

        schedule.add_systems(purchase_system);
    
    
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
        flush_to_db_schedule.add_systems(flush_component_to_db::<ItemQuery>);
        flush_to_db_schedule.add_systems(flush_component_to_db::<PurchaseItemQuery>);
        flush_to_db_schedule.run(&mut world);

        let mut commit_schedule = Schedule::default();
        commit_schedule.add_systems(|db : ResMut<AnyDatabaseResource>|{

            block_on(async {
                let db_handle = db.get_connection();
                let tr_option = &mut (*db_handle).write().unwrap().tr;
                let tr = tr_option.take().unwrap();
                tr.commit().await.unwrap();
            });
        });
        commit_schedule.run(&mut world);
    
    
        println!("done"); 


        // See the updated data
        {
            let mut reader = IntoSystem::into_system(|db : ResMut<AnyDatabaseResource>|{
                block_on( async {
                    let db_handle = db.get_connection();
                    let conn =&(*db_handle).write().unwrap().pool;
                    let items = sqlx::query_as::<_, PurchasedItem>("SELECT * FROM purchased_items").fetch_all(conn).await.unwrap();
                    println!("purchased_items: {:?}", items);
                });
            });

            reader.initialize(&mut world);
            reader.run((), &mut world);
        }
    
}

#[tokio::main]
async fn main() {
    run().await;
}
