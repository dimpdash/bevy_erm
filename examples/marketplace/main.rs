
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
    mut db_query: DatabaseQuery<&ItemQuery>,
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
    mut db_query_purchased: DatabaseQuery<&PurchaseItemQuery>,
) {
    println!("purchase system");
    for event in events.read() {
        println!("purchased item: {:?}", event.item);
        let purchased_item = PurchasedItem {
            item: event.item,
            buyer: event.purchaser,
        };

        db_query_purchased.create(purchased_item);
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

fn print_items_table( mut items: DatabaseQuery<&ItemQuery>, db_entity_query : Query<&DatabaseEntity>) {
    let items = items.load_components(ItemQuery::load_all())
    
    let items = items.load_components_and_entity(ItemQuery::load_all()).unwrap().into_iter().map(| (entity, item) | {
        let db_entity = db_entity_query.get(entity).unwrap();
        (db_entity, item)
    }).collect::<Vec<_>>();

    let mut items_table = prettytable::Table::new();
    items_table.add_row(row!["id", "seller_id", "name", "price"]);
    for (db_entity, item) in items {
        items_table.add_row(row![db_entity.id, item.seller_id.id, item.name, item.price]);
    }

    println!("Items");
    items_table.printstd();
}

fn print_purchased_items_table( mut purchased_items: DatabaseQuery<&PurchaseItemQuery>, db_entity_query : Query<&DatabaseEntity>) {
    let purchased_items = purchased_items.load_components_and_entity(PurchaseItemQuery::load_all()).unwrap().into_iter().map(| (entity, item) | {
        let db_entity = db_entity_query.get(entity).unwrap();
        (db_entity, item)
    }).collect::<Vec<_>>();

    let mut purchased_items_table = prettytable::Table::new();
    purchased_items_table.add_row(row!["id", "item", "buyer"]);
    for (db_entity, item) in purchased_items {
        purchased_items_table.add_row(row![db_entity.id, item.item.id, item.buyer.id]);
    }

    println!("Purchased Items");
    purchased_items_table.printstd();
}

fn print_tables(mut users : DatabaseQuery<&UserQuery>, mut items: DatabaseQuery<&ItemQuery>, mut purchased_items: DatabaseQuery<&PurchaseItemQuery>, mut buyers: DatabaseQuery<&BuyerQuery>, mut sellers: DatabaseQuery<&SellerQuery>, db_entity_query : Query<&DatabaseEntity>) {
    let users = {
        let users = users.load_components_and_entity(UserQuery::load_all()).unwrap();

        let buyers = buyers.load_components_and_entity(BuyerQuery::load_all()).unwrap();
        let sellers = sellers.load_components_and_entity(SellerQuery::load_all()).unwrap();
        
        users.into_iter().map(|(entity, user)| {
            let buyer = buyers.iter().find(|(buyer_entity, _) | buyer_entity == &entity).is_some();
            let seller = sellers.iter().find(|(seller_entity, _) | seller_entity == &entity).is_some();
            let db_entity = db_entity_query.get(entity).unwrap();
            (db_entity, user, buyer, seller)
        }).collect::<Vec<_>>()
    };
    let items = items.load_components_and_entity(ItemQuery::load_all()).unwrap().into_iter().map(| (entity, item) | {
        let db_entity = db_entity_query.get(entity).unwrap();
        (db_entity, item)
    }).collect::<Vec<_>>();
    let purchased_items = purchased_items.load_components_and_entity(PurchaseItemQuery::load_all()).unwrap().into_iter().map(| (entity, item) | {
        let db_entity = db_entity_query.get(entity).unwrap();
        (db_entity, item)
    }).collect::<Vec<_>>();



    let mut users_table = prettytable::Table::new();
    users_table.add_row(row!["id", "name", "buyer", "seller"]);
    for (db_entity, user, buyer, seller) in users {
        users_table.add_row(row![db_entity.id, user.name, buyer, seller]);
    }

    let mut items_table = prettytable::Table::new();
    items_table.add_row(row!["id", "seller_id", "name", "price"]);
    for (db_entity, item) in items {
        items_table.add_row(row![db_entity.id, item.seller_id.id, item.name, item.price]);
    }

    let mut purchased_items_table = prettytable::Table::new();
    purchased_items_table.add_row(row!["id", "item", "buyer"]);
    for (db_entity, item) in purchased_items {
        purchased_items_table.add_row(row![db_entity.id, item.item.id, item.buyer.id]);
    }

    println!("Users");
    users_table.printstd();
    println!("Items");
    items_table.printstd();
    println!("Purchased Items");
    purchased_items_table.printstd();


   

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

fn commit_transaction(db: ResMut<AnyDatabaseResource>){
    block_on(async {
        let db_handle = db.get_connection();
        let tr_option = &mut (*db_handle).write().unwrap().tr;
        let tr = tr_option.take().unwrap();
        tr.commit().await.unwrap();
    });
}

fn start_new_transaction(db: ResMut<AnyDatabaseResource>){
    block_on(async {
        let db_handle = db.get_connection();
        let new_transaction = {
            let pool = &(*db_handle).write().unwrap().pool;
            pool.begin().await.unwrap()
        };
        let tr_option = &mut (*db_handle).write().unwrap().tr;
        *tr_option = Some(new_transaction);
    });
}

fn runner(mut app: App) {
    let mut pre_pre_startup_schedule = schedule::Schedule::default();
    pre_pre_startup_schedule.add_systems(preload_events);
    pre_pre_startup_schedule.run(&mut app.world);

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
    commit_schedule.add_systems(commit_transaction);
    commit_schedule.run(&mut app.world);

    println!("done");

    let mut new_transation_schedule = Schedule::default();
    new_transation_schedule.add_systems(start_new_transaction);
    new_transation_schedule.run(&mut app.world);

    let mut end_schedule = Schedule::default();
    end_schedule.add_systems(print_purchased_items_table);
    end_schedule.add_systems(print_items_table);
    end_schedule.run(&mut app.world);
}

#[tokio::main]
async fn main() {
    App::new()
        .set_runner(runner)
        .add_plugins(MarketplacePlugin)
        .run();
}
