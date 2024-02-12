use bevy_ecs::{component::Component, prelude::*, schedule};
use bevy_erm::*;

use async_trait::async_trait;

use bevy_app::prelude::*;
use futures::executor::block_on;
use sqlx::{FromRow, Row};

#[derive(Event, Debug)]
pub struct Purchase {
    pub item: DatabaseEntity,
    pub purchaser: DatabaseEntity,
}

/**
 * Creates a new item to sell
 */
#[derive(Event, Debug)]
pub struct Sell {
    pub item: DatabaseEntity,
    pub seller: DatabaseEntity,
}

#[derive(Event, Debug)]
pub struct GetSellerItems {
    pub seller: DatabaseEntity,
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
            seller_id: DatabaseEntity {
                id: row.try_get("seller_id")?,
                persisted: true.into(),
                dirty: false,
            },
            name: row.get("name"),
            price: row.get("price"),
        })
    }
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
struct User {
    #[allow(dead_code)]
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
            item: DatabaseEntity {
                id: row.try_get("item")?,
                persisted: true.into(),
                dirty: false,
            },
            buyer: DatabaseEntity {
                id: row.try_get("buyer")?,
                persisted: true.into(),
                dirty: false,
            },
        })
    }
}

struct ItemQuery {}

impl ItemQuery {
    fn load_items_of_seller(
        seller: DatabaseEntity,
    ) -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, MarketItem)>, ()>
    {
        move |conn: &mut sqlx::SqliteConnection| {
            let items = block_on(
                sqlx::query("SELECT id, seller_id, name, price FROM items WHERE seller_id = ?")
                    .bind(seller.id)
                    .fetch_all(conn),
            )
            .unwrap();

            let items = items
                .into_iter()
                .map(|row| {
                    let id = row.get("id");
                    let seller_id = row.get("seller_id");
                    let name = row.get("name");
                    let price = row.get("price");

                    (
                        DatabaseEntity {
                            id,
                            persisted: true.into(),
                            dirty: false,
                        },
                        MarketItem {
                            seller_id: DatabaseEntity {
                                id: seller_id,
                                persisted: true.into(),
                                dirty: false,
                            },
                            name,
                            price,
                        },
                    )
                })
                .collect();

            Ok(items)
        }
    }
}

#[async_trait]
impl DatabaseQueryInfo for ItemQuery {
    type Component = MarketItem;
    type Database = AnyDatabaseResource;
    type Index = DatabaseEntityIndex;

    fn get_component<'c, E>(conn: E, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let item = block_on(
            sqlx::query_as::<_, MarketItem>("SELECT item FROM items WHERE id = ?")
                .bind(db_entity.id as i32)
                .fetch_one(conn),
        )
        .unwrap();
        Ok(item)
    }

    async fn update_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query("UPDATE items SET seller_id = ?, name = ?, price = ? WHERE id = ?")
            .bind(component.seller_id.id)
            .bind(component.name.clone())
            .bind(component.price)
            .bind(db_entity.id)
            .execute(tr)
            .await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn insert_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query("INSERT INTO items (id, seller_id, name, price) VALUES (?, ?, ?, ?)")
            .bind(db_entity.id)
            .bind(component.seller_id.id)
            .bind(component.name.clone())
            .bind(component.price)
            .execute(tr)
            .await;

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntity) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query("DELETE FROM items WHERE id = ?")
            .bind(db_entity.id)
            .execute(tr)
            .await;
        match r {
            Ok(_) => Ok(()),
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
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let item = block_on(
            sqlx::query_as::<_, PurchasedItem>("SELECT item FROM purchased_items WHERE id = ?")
                .bind(db_entity.id)
                .fetch_one(conn),
        )
        .unwrap();
        Ok(item)
    }

    async fn update_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query("UPDATE purchased_items SET item = ?, buyer = ? WHERE id = ?")
            .bind(component.item.id)
            .bind(component.buyer.id)
            .bind(db_entity.id)
            .execute(tr)
            .await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn insert_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query("INSERT INTO purchased_items (id, item, buyer) VALUES (?, ?, ?)")
            .bind(db_entity.id)
            .bind(component.item.id)
            .bind(component.buyer.id)
            .execute(tr)
            .await;

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntity) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query("DELETE FROM purchased_items WHERE id = ?")
            .bind(db_entity.id)
            .execute(tr)
            .await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

fn get_seller_items(
    mut events: EventReader<GetSellerItems>,
    mut db_query: DatabaseQuery<ItemQuery>,
) {
    println!("get seller items system");
    for event in events.read() {
        let seller = event.seller;
        let items = db_query.load_components(ItemQuery::load_items_of_seller(seller));
        println!("seller items: {:?}", items);
    }
}

fn purchase_system(
    mut events: EventReader<Purchase>,
    mut db_query_purchased: DatabaseQuery<PurchaseItemQuery>,
) {
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

fn create_tables(db: ResMut<AnyDatabaseResource>) {
    let db_handle = db.get_connection();
    let conn = &(*db_handle).write().unwrap().pool;

    block_on(async {
        // create the tables
        // market items table
        sqlx::query("CREATE TABLE items (id INTEGER PRIMARY KEY, seller_id INTEGER, name TEXT, price INTEGER)")
            .execute(conn).await.unwrap();

        // purchased items table
        sqlx::query(
            "CREATE TABLE purchased_items (id INTEGER PRIMARY KEY, item INTEGER, buyer INTEGER)",
        )
        .execute(conn)
        .await
        .unwrap();

        // users table
        sqlx::query(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, buyer BOOLEAN, seller BOOLEAN)",
        )
        .execute(conn)
        .await
        .unwrap();


        // populate one buyer and one seller
        sqlx::query("INSERT INTO users (id, name, buyer, seller) VALUES (?, 'buyer', 1, 0)")
            .bind(PURCHASER_ID)
            .execute(conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO users (id, name, buyer, seller) VALUES (?, 'seller', 0, 1)")
            .bind(SELLER_ID)
            .execute(conn)
            .await
            .unwrap();

        // add one item to the market
        sqlx::query("INSERT INTO items (id, seller_id, name, price) VALUES (?, ?, 'corn', 100)")
            .bind(MARKET_ITEM_ID)
            .bind(SELLER_ID)
            .execute(conn)
            .await
            .unwrap();
    });
}

fn events_available<E: Event>(mut events: EventReader<E>) -> bool {
    let not_empty = !events.is_empty();
    events.clear(); // prevent old events from retriggering as weren't read
    not_empty
}

const PURCHASER_ID: i64 = 1;
const SELLER_ID: i64 = 2;
const MARKET_ITEM_ID: i64 = 3;

pub struct MarketplacePlugin;

impl Plugin for MarketplacePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Purchase>()
            .add_event::<Sell>()
            .add_event::<GetSellerItems>()
            .init_resource::<AnyDatabaseResource>()
            .add_systems(Startup, create_tables)
            .add_systems(Update, purchase_system)
            .add_systems(Update, get_seller_items);
    }
}

fn preload_events(mut purchase_events: EventWriter<Purchase>,
    mut get_seller_items: EventWriter<GetSellerItems>) {

        // add the triggering purchase event
        purchase_events.send(Purchase {
            purchaser: DatabaseEntity {
                id: PURCHASER_ID,
                persisted: true.into(),
                dirty: false,
            },
            item: DatabaseEntity {
                id: MARKET_ITEM_ID,
                persisted: true.into(),
                dirty: false,
            },
        });
    
        get_seller_items.send(GetSellerItems {
            seller: DatabaseEntity {
                id: SELLER_ID,
                persisted: true.into(),
                dirty: false,
            },
        });
}

fn runner(mut app: App) {
    let mut schedule = schedule::Schedule::default();
    schedule.add_systems(preload_events);
    schedule.run(&mut app.world);

    let mut is_sell_events = IntoSystem::into_system(events_available::<Sell>);
    let mut is_purchase_events = IntoSystem::into_system(events_available::<Purchase>);
    let mut is_get_seller_items_events =
        IntoSystem::into_system(events_available::<GetSellerItems>);

    is_sell_events.initialize(&mut app.world);
    is_purchase_events.initialize(&mut app.world);
    is_get_seller_items_events.initialize(&mut app.world);

    let mut still_events_to_read = |world: &mut World| -> bool {
        is_sell_events.run((), world)
            || is_purchase_events.run((), world)
            || is_get_seller_items_events.run((), world)
    };

    while still_events_to_read(&mut app.world) {
        println!("==== running iteration ====");
        app.update();
    }
    println!("===========================");

    let mut flush_to_db_schedule = Schedule::default();
    flush_to_db_schedule.add_systems(flush_component_to_db::<ItemQuery>);
    flush_to_db_schedule.add_systems(flush_component_to_db::<PurchaseItemQuery>);
    flush_to_db_schedule.run(&mut app.world);

    let mut commit_schedule = Schedule::default();
    commit_schedule.add_systems(|db: ResMut<AnyDatabaseResource>| {
        block_on(async {
            let db_handle = db.get_connection();
            let tr_option = &mut (*db_handle).write().unwrap().tr;
            let tr = tr_option.take().unwrap();
            tr.commit().await.unwrap();
        });
    });
    commit_schedule.run(&mut app.world);

    println!("done");

    // See the updated data
    {
        let mut reader = IntoSystem::into_system(|db: ResMut<AnyDatabaseResource>| {
            block_on(async {
                let db_handle = db.get_connection();
                let conn = &(*db_handle).write().unwrap().pool;
                let items = sqlx::query_as::<_, PurchasedItem>("SELECT * FROM purchased_items")
                    .fetch_all(conn)
                    .await
                    .unwrap();
                println!("purchased_items: {:?}", items);
            });
        });

        reader.initialize(&mut app.world);
        reader.run((), &mut app.world);
    }
}

#[tokio::main]
async fn main() {
    App::new()
        .set_runner(runner)
        .add_plugins(MarketplacePlugin)
        .run();
}
