mod components;
mod queries;

use bevy_ecs::prelude::*;
use bevy_erm::*;
#[macro_use]
extern crate prettytable;

use bevy_app::{prelude::*, AppExit};
use components::*;
use futures::executor::block_on;
use queries::*;

#[derive(Event, Debug)]
pub struct Purchase {
    pub item: DatabaseEntityId,
    pub purchaser: DatabaseEntityId,
    pub request: RequestId,
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

#[derive(Event)]
pub struct PurchaseResponse{
    request: RequestId,
}

fn purchase_system(
    mut purchases: EventReader<Purchase>,
    db_query_purchased: DatabaseQuery<&PurchaseItemQuery>,
    item_query: DatabaseQuery<&ItemQuery>,
    purchaser_query: DatabaseQuery<&UserQuery>,
    seller_query: DatabaseQuery<&UserQuery>,
    mut response: EventWriter<PurchaseResponse>,
) {
    if purchases.is_empty() {
        return;
    }
    println!("Processing purchase events");
    for purchase in purchases.read() {
        let item = item_query.get(&(purchase.item, purchase.request)).unwrap();
        let seller_name = seller_query.get(&(item.seller_id, purchase.request)).unwrap().name.clone();
        let buyer_name = purchaser_query.get(&(purchase.purchaser, purchase.request)).unwrap().name.clone();
        println!(
            "\t{:} purchases {:} from {:}",
            buyer_name, item.name, seller_name
        );
        let purchased_item = PurchasedItem {
            item: purchase.item,
            buyer: purchase.purchaser,
        };

        db_query_purchased.create(purchased_item, purchase.request).unwrap();

        response.send(PurchaseResponse{request: purchase.request});
    }
}

fn print_tables(mut print_table: EventWriter<PrintTable>) {
    print_table.send(PrintTable);
}

fn create_tables(db: Res<AnyDatabaseResource>) {
    let db_handle = db.get_connection();
    let conn = &(*db_handle).write().unwrap().pool;

    block_on(async {
        println!("Creating tables");

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
        sqlx::query(
            "INSERT INTO users (id, name, buyer, seller) VALUES (?, 'Bob The Buyer', 1, 0)",
        )
        .bind(PURCHASER_ID)
        .execute(conn)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO users (id, name, buyer, seller) VALUES (?, 'Alice The Seller', 0, 1)",
        )
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

#[derive(Event)]
pub struct PrintTable;

fn print_items_table(items: DatabaseQuery<&ItemQuery>, print_table: EventReader<PrintTable>) {
    if print_table.is_empty() {
        return;
    }
    let items = items
        .load_components::<(&DatabaseEntity, &MarketItem)>(ItemQuery::load_all(-1))
        .unwrap();

    let mut items_table = prettytable::Table::new();
    items_table.add_row(row!["id", "seller_id", "name", "price"]);
    for (db_entity, item) in items {
        items_table.add_row(row![db_entity.id, item.seller_id, item.name, item.price]);
    }

    println!("Items");
    items_table.printstd();
}

fn print_purchased_items_table(
    purchased_items: DatabaseQuery<&PurchaseItemQuery>,
    print_table: EventReader<PrintTable>,
) {
    if print_table.is_empty() {
        return;
    }
    let purchased_items: Vec<(&DatabaseEntity, &PurchasedItem)> = purchased_items
        .load_components::<(&DatabaseEntity, &PurchasedItem)>(PurchaseItemQuery::load_all(-1))
        .unwrap();

    let mut purchased_items_table = prettytable::Table::new();
    purchased_items_table.add_row(row!["id", "item", "buyer"]);
    for (db_entity, item) in purchased_items {
        purchased_items_table.add_row(row![db_entity.id, item.item, item.buyer]);
    }

    println!("Purchased Items");
    purchased_items_table.printstd();
}

fn print_users_table(
    users: DatabaseQuery<&UserQuery>,
    buyers: DatabaseQuery<&BuyerQuery>,
    sellers: DatabaseQuery<&SellerQuery>,
    print_table: EventReader<PrintTable>,
) {
    if print_table.is_empty() {
        return;
    }
    let users = {
        let users = users
            .load_components::<(Entity, &DatabaseEntity, &User)>(UserQuery::load_all(-1))
            .unwrap();

        let buyers = buyers
            .load_components::<(Entity, &Buyer)>(BuyerQuery::load_all(-1))
            .unwrap();
        let sellers = sellers
            .load_components::<(Entity, &Seller)>(SellerQuery::load_all(-1))
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

const PURCHASER_ID: i64 = 1;
const SELLER_ID: i64 = 2;
const MARKET_ITEM_ID: i64 = 3;

fn flush_purchase(
    mut purchase_events: EventReader<PurchaseResponse>,
    mut flush: EventWriter<FlushEvent>,
) {
    for purchase_event in purchase_events.read() {
        println!("Flushing purchase events");
        flush.send(FlushEvent{request: purchase_event.request});
    }
}

pub struct MarketplacePlugin;

impl Plugin for MarketplacePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Purchase>()
            .add_event::<Sell>()
            .add_event::<GetSellerItems>()
            .add_event::<PurchaseResponse>()
            .add_event::<PrintTable>()
            .add_systems(PreStartup, preload_events)
            .add_systems(Startup, create_tables)
            .add_systems(Startup, print_tables.after(create_tables))
            .add_systems(Startup, print_items_table.after(print_tables))
            .add_systems(Startup, print_users_table.after(print_tables))
            .add_systems(Startup, print_purchased_items_table.after(print_tables))
            .add_systems(Update, purchase_system)
            .add_systems(Update, flush_purchase)
            .add_systems(PostUpdate, print_items_table)
            .add_systems(PostUpdate, print_users_table)
            .add_systems(PostUpdate, print_purchased_items_table)
            .add_systems(PostUpdate, should_exit);
    }
}

fn preload_events(
    mut purchase_events: EventWriter<Purchase>,
    _get_seller_items: EventWriter<GetSellerItems>,
) {
    println!("Preloading events:");

    // create two purchase events

    let purchase_event = Purchase {
        purchaser: PURCHASER_ID,
        item: MARKET_ITEM_ID,
        request: 0,
    };

    println!(
        "\tPreloading purchase event:\n\t\tbuyer {:?}, item {:?}",
        purchase_event.purchaser, purchase_event.item
    );
    purchase_events.send(purchase_event);

    let purchase_event = Purchase {
        purchaser: PURCHASER_ID,
        item: MARKET_ITEM_ID,
        request: 1,
    };

    println!(
        "\tPreloading purchase event:\n\t\tbuyer {:?}, item {:?}",
        purchase_event.purchaser, purchase_event.item
    );
    purchase_events.send(purchase_event);

    // let get_seller_items_event = GetSellerItems {
    //     seller: DatabaseEntity {
    //         id: SELLER_ID,
    //         persisted: true.into(),
    //         dirty: false,
    //     },
    // };

    // println!(
    //     "\tPreloading get seller items event:\n\t\tseller {:?}",
    //     get_seller_items_event.seller.id
    // );
    // get_seller_items.send(get_seller_items_event);

    println!();
}

fn should_exit(purchase_events: EventReader<PurchaseResponse>, mut exit: EventWriter<AppExit>) {
    if !purchase_events.is_empty() {
        exit.send(AppExit);
    }
}

#[tokio::main]
async fn main() {
    let mut erm_plugin = EntityRelationMapperPlugin::new();
    erm_plugin
        .add_flush_system(flush_component_to_db::<ItemQuery>)
        .add_flush_system(flush_component_to_db::<UserQuery>);

    App::new()
        .add_plugins(erm_plugin)
        .add_plugins(MarketplacePlugin)
        .run();
}
