
mod queries;
mod components;

use bevy_ecs::{component::Component, prelude::*, schedule};
use bevy_erm::*;
#[macro_use] extern crate prettytable;
use async_trait::async_trait;


use bevy_app::prelude::*;
use futures::executor::block_on;
use sqlx::{FromRow, Row};
use queries::*;
use components::*;

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

                let mut table = prettytable::Table::new();
                table.add_row(row!["item", "buyer"]);
                for item in items {
                    table.add_row(row![item.item.id, item.buyer.id]);
                }

                println!("Purchased Items");
                table.printstd();
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
