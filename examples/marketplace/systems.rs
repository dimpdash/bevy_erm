use bevy_ecs::prelude::*;
use bevy_erm::*;

use bevy_app::{prelude::*, AppExit};

use crate::components::*;
use crate::events::*;
use crate::queries::*;
use crate::resources::*;
use futures::executor::block_on;

pub fn purchase_system(
    mut purchases: EventReader<Purchase>,
    db_query_purchased: DatabaseQuery<&PurchaseItemQuery>,
    item_query: DatabaseQuery<&ItemQuery>,
    purchaser_query: DatabaseQuery<&UserQuery>,
    seller_query: DatabaseQuery<&UserQuery>,
    mut response: EventWriter<PurchaseResponse>,
) {
    block_on(async {
        println!("Processing purchase events");
        for purchase in purchases.read() {
            let item = item_query
                .get(&(purchase.item, purchase.request))
                .await
                .unwrap();
            let seller_name = seller_query
                .get(&(item.seller_id, purchase.request))
                .await
                .unwrap()
                .name
                .clone();
            let buyer_name = purchaser_query
                .get(&(purchase.purchaser, purchase.request))
                .await
                .unwrap()
                .name
                .clone();
            println!(
                "\t{:} purchases {:} from {:}",
                buyer_name, item.name, seller_name
            );
            let purchased_item = PurchasedItem {
                item: purchase.item,
                buyer: purchase.purchaser,
            };

            db_query_purchased
                .create(purchased_item, purchase.request)
                .await
                .unwrap();

            response.send(PurchaseResponse {
                request: purchase.request,
            });
        }
        println!("Finished processing purchase events");
    });
}

pub fn create_tables(db: Res<AnyDatabaseResource>, _print_tables: EventWriter<PrintTable>) {
    println!("Creating tables");
    let request = db.start_new_transaction();
    block_on(async {
        // let db_handle = db.get_connection();
        // let pool = db_handle.pool.write().unwrap();
        // let mut conn = pool.acquire().await.unwrap();
        let conn = db.get_transaction(request);
        let mut guard = conn.lock().await;
        let tr = guard.a.as_mut().unwrap();

        // create the tables
        // market items table
        sqlx::query("CREATE TABLE items (id INTEGER PRIMARY KEY, seller_id INTEGER, name TEXT, price INTEGER)")
            .execute(&mut **tr).await.unwrap();

        // purchased items table
        sqlx::query(
            "CREATE TABLE purchased_items (id INTEGER PRIMARY KEY, item INTEGER, buyer INTEGER)",
        )
        .execute(&mut **tr)
        .await
        .unwrap();

        // users table
        sqlx::query(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, buyer BOOLEAN, seller BOOLEAN)",
        )
        .execute(&mut **tr)
        .await
        .unwrap();

        // populate one buyer and one seller
        sqlx::query(
            "INSERT INTO users (id, name, buyer, seller) VALUES (?, 'Bob The Buyer', 1, 0)",
        )
        .bind(PURCHASER_ID)
        .execute(&mut **tr)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO users (id, name, buyer, seller) VALUES (?, 'Alice The Seller', 0, 1)",
        )
        .bind(SELLER_ID)
        .execute(&mut **tr)
        .await
        .unwrap();

        // add one item to the market
        sqlx::query("INSERT INTO items (id, seller_id, name, price) VALUES (?, ?, 'corn', 100)")
            .bind(MARKET_ITEM_ID)
            .bind(SELLER_ID)
            .execute(&mut **tr)
            .await
            .unwrap();
    });

    db.commit_transaction(request);

    // let request = db.start_new_transaction();
    // print_tables.send(PrintTable { request });
}

pub fn print_items_table(
    items: DatabaseQuery<&ItemQuery>,
    mut print_table_events: EventReader<PrintTable>,
) {
    block_on(async {
        for print_table in print_table_events.read() {
            let items = items
                .load_components::<(&DatabaseEntity, &MarketItem), ItemQueryLoadAll>(
                    print_table.request,
                    ItemQueryLoadAll(print_table.request),
                )
                .await
                .unwrap();

            let mut items_table = prettytable::Table::new();
            items_table.add_row(row!["id", "seller_id", "name", "price"]);
            for (db_entity, item) in items {
                items_table.add_row(row![db_entity.id, item.seller_id, item.name, item.price]);
            }

            println!("Items");
            items_table.printstd();
        }
    });
}

pub fn print_purchased_items_table(
    purchased_items: DatabaseQuery<&PurchaseItemQuery>,
    mut print_table_events: EventReader<PrintTable>,
) {
    block_on(async {
        for print_table in print_table_events.read() {
            let purchased_items = purchased_items
                .load_components::<(&DatabaseEntity, &PurchasedItem), PurchaseItemQueryLoadAll>(
                    print_table.request,
                    PurchaseItemQueryLoadAll(print_table.request),
                )
                .await
                .unwrap();

            let mut purchased_items_table = prettytable::Table::new();
            purchased_items_table.add_row(row!["id", "item", "buyer"]);
            for (db_entity, item) in purchased_items {
                purchased_items_table.add_row(row![db_entity.id, item.item, item.buyer]);
            }

            println!("Purchased Items");
            purchased_items_table.printstd();
        }
    });
}

pub fn print_users_table(
    users: DatabaseQuery<&UserQuery>,
    buyers: DatabaseQuery<&BuyerQuery>,
    sellers: DatabaseQuery<&SellerQuery>,
    mut print_table_events: EventReader<PrintTable>,
) {
    block_on(async {
        for print_table in print_table_events.read() {
            let users = {
                let users = users
                    .load_components::<(Entity, &DatabaseEntity, &User), UserQueryLoadAll>(
                        print_table.request,
                        UserQueryLoadAll(print_table.request),
                    )
                    .await
                    .unwrap();

                let buyers = buyers
                    .load_components::<(Entity, &Buyer), BuyerQueryLoadAll>(
                        print_table.request,
                        BuyerQueryLoadAll(print_table.request),
                    )
                    .await
                    .unwrap();

                let sellers = sellers
                    .load_components::<(Entity, &Seller), SellerQueryLoadAll>(
                        print_table.request,
                        SellerQueryLoadAll(print_table.request),
                    )
                    .await
                    .unwrap();
                users
                    .into_iter()
                    .map(|(entity, db_entity, user)| {
                        let buyer = buyers
                            .iter()
                            .any(|(buyer_entity, _)| buyer_entity == &entity);
                        let seller = sellers
                            .iter()
                            .any(|(seller_entity, _)| seller_entity == &entity);
                        (db_entity, user, buyer, seller)
                    })
                    .collect::<Vec<_>>()
            };

            let mut users_table = prettytable::Table::new();
            users_table.add_row(row!["id", "name", "buyer", "seller"]);
            for (db_entity, user, buyer, seller) in users {
                users_table.add_row(row![db_entity.id, user.name, buyer, seller]);
            }

            println!("Users");
            users_table.printstd();
        }
    });
}

const PURCHASER_ID: i64 = 1;
const SELLER_ID: i64 = 2;
const MARKET_ITEM_ID: i64 = 3;

pub fn flush_purchase(
    mut purchase_events: EventReader<PurchaseResponse>,
    mut flush: EventWriter<FlushEvent>,
    mut webserver: ResMut<WebServer>,
) {
    for purchase_event in purchase_events.read() {
        webserver.respond_purchase_event(purchase_event);
        flush.send(FlushEvent {
            request: purchase_event.request,
        });
    }
}

pub fn poll_webserver_for_requests(
    mut purchase_events: EventWriter<Purchase>,
    _get_seller_items: EventWriter<GetSellerItems>,
    db: Res<AnyDatabaseResource>,
    webserver: ResMut<WebServer>,
    exit: EventReader<AppExit>,
) {
    if !exit.is_empty() {
        return;
    }

    // create two purchase events
    // println!("====================================");
    // println!("Polling webserver for requests");
    while webserver.is_messages_to_send() {
        if let Some(request) = db.try_start_new_transaction() {
            let purchase_event = Purchase {
                purchaser: DatabaseEntityId(PURCHASER_ID),
                item: DatabaseEntityId(MARKET_ITEM_ID),
                request,
            };

            println!(
                "\tHandling purchase event:\n\t\tbuyer {}, item {}, request {}",
                purchase_event.purchaser, purchase_event.item, purchase_event.request
            );
            purchase_events.send(purchase_event);
        } else {
            break;
        }
    }
    // println!();
}

pub fn should_exit(
    mut exit: EventWriter<AppExit>,
    mut print_tables: EventWriter<PrintTable>,
    db: Res<AnyDatabaseResource>,
    webserver: Res<WebServer>,
) {
    if webserver.should_exit() {
        println!();
        println!("============ Exiting ==============");
        exit.send(AppExit);
        let request = db.start_new_transaction();

        println!("Tables after handling requests");
        println!();
        print_tables.send(PrintTable { request });
    }
}

pub fn runner(mut app: App) {
    loop {
        app.update();

        if !app
            .world
            .get_resource::<Events<AppExit>>()
            .unwrap()
            .is_empty()
        {
            break;
        }
    }
}
