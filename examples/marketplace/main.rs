mod components;
mod queries;

use std::any::TypeId;

use bevy_ecs::{component::ComponentId, prelude::*};
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
pub struct PurchaseResponse {
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
        let seller_name = seller_query
            .get(&(item.seller_id, purchase.request))
            .unwrap()
            .name
            .clone();
        let buyer_name = purchaser_query
            .get(&(purchase.purchaser, purchase.request))
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
            .unwrap();

        println!("Sending purchase response");
        response.send(PurchaseResponse {
            request: purchase.request,
        });
    }
}

#[allow(dead_code)]
fn print_tables(mut print_tables: EventWriter<PrintTable>, db: Res<AnyDatabaseResource>) {
    println!("PRINTING TABLES");
    let request = db.start_new_transaction();
    print_tables.send(PrintTable { request });
}

fn create_tables(db: Res<AnyDatabaseResource>, _print_tables: EventWriter<PrintTable>) {
    let request = db.start_new_transaction();
    block_on(async {
        // let db_handle = db.get_connection();
        // let pool = db_handle.pool.write().unwrap();
        // let mut conn = pool.acquire().await.unwrap();
        get_transaction!(conn, request, db);

        // create the tables
        // market items table
        sqlx::query("CREATE TABLE items (id INTEGER PRIMARY KEY, seller_id INTEGER, name TEXT, price INTEGER)")
            .execute(&mut **conn).await.unwrap();

        // purchased items table
        sqlx::query(
            "CREATE TABLE purchased_items (id INTEGER PRIMARY KEY, item INTEGER, buyer INTEGER)",
        )
        .execute(&mut **conn)
        .await
        .unwrap();

        // users table
        sqlx::query(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, buyer BOOLEAN, seller BOOLEAN)",
        )
        .execute(&mut **conn)
        .await
        .unwrap();

        // populate one buyer and one seller
        sqlx::query(
            "INSERT INTO users (id, name, buyer, seller) VALUES (?, 'Bob The Buyer', 1, 0)",
        )
        .bind(PURCHASER_ID)
        .execute(&mut **conn)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO users (id, name, buyer, seller) VALUES (?, 'Alice The Seller', 0, 1)",
        )
        .bind(SELLER_ID)
        .execute(&mut **conn)
        .await
        .unwrap();

        // add one item to the market
        sqlx::query("INSERT INTO items (id, seller_id, name, price) VALUES (?, ?, 'corn', 100)")
            .bind(MARKET_ITEM_ID)
            .bind(SELLER_ID)
            .execute(&mut **conn)
            .await
            .unwrap();
    });
    db.commit_transaction(request);

    // let request = db.start_new_transaction();
    // print_tables.send(PrintTable { request });
}

#[derive(Event)]
pub struct PrintTable {
    request: RequestId,
}

fn print_items_table(
    items: DatabaseQuery<&ItemQuery>,
    mut print_table_events: EventReader<PrintTable>,
) {
    for print_table in print_table_events.read() {
        let items = items
            .load_components::<(&DatabaseEntity, &MarketItem)>(
                print_table.request,
                ItemQuery::load_all(print_table.request),
            )
            .unwrap();

        let mut items_table = prettytable::Table::new();
        items_table.add_row(row!["id", "seller_id", "name", "price"]);
        for (db_entity, item) in items {
            items_table.add_row(row![db_entity.id, item.seller_id, item.name, item.price]);
        }

        println!("Items");
        items_table.printstd();
    }
}

fn print_purchased_items_table(
    purchased_items: DatabaseQuery<&PurchaseItemQuery>,
    mut print_table_events: EventReader<PrintTable>,
) {
    for print_table in print_table_events.read() {
        let purchased_items: Vec<(&DatabaseEntity, &PurchasedItem)> = purchased_items
            .load_components::<(&DatabaseEntity, &PurchasedItem)>(
                print_table.request,
                PurchaseItemQuery::load_all(print_table.request),
            )
            .unwrap();

        let mut purchased_items_table = prettytable::Table::new();
        purchased_items_table.add_row(row!["id", "item", "buyer"]);
        for (db_entity, item) in purchased_items {
            purchased_items_table.add_row(row![db_entity.id, item.item, item.buyer]);
        }

        println!("Purchased Items");
        purchased_items_table.printstd();
    }
}

fn print_users_table(
    users: DatabaseQuery<&UserQuery>,
    buyers: DatabaseQuery<&BuyerQuery>,
    sellers: DatabaseQuery<&SellerQuery>,
    mut print_table_events: EventReader<PrintTable>,
) {
    for print_table in print_table_events.read() {
        let users = {
            let users = users
                .load_components::<(Entity, &DatabaseEntity, &User)>(
                    print_table.request,
                    UserQuery::load_all(print_table.request),
                )
                .unwrap();

            let buyers = buyers
                .load_components::<(Entity, &Buyer)>(
                    print_table.request,
                    BuyerQuery::load_all(print_table.request),
                )
                .unwrap();
            let sellers = sellers
                .load_components::<(Entity, &Seller)>(
                    print_table.request,
                    SellerQuery::load_all(print_table.request),
                )
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
}

const PURCHASER_ID: i64 = 1;
const SELLER_ID: i64 = 2;
const MARKET_ITEM_ID: i64 = 3;

fn flush_purchase(
    mut purchase_events: EventReader<PurchaseResponse>,
    mut flush: EventWriter<FlushEvent>,
) {
    for purchase_event in purchase_events.read() {
        flush.send(FlushEvent {
            request: purchase_event.request,
        });
    }
}

fn preload_events(
    mut purchase_events: EventWriter<Purchase>,
    _get_seller_items: EventWriter<GetSellerItems>,
    db: Res<AnyDatabaseResource>,
) {
    println!("Preloading events:");

    // create two purchase events

    for _ in 0..2 {
        let purchase_event = Purchase {
            purchaser: DatabaseEntityId(PURCHASER_ID),
            item: DatabaseEntityId(MARKET_ITEM_ID),
            request: db.start_new_transaction(),
        };

        println!(
            "\tPreloading purchase event:\n\t\tbuyer {}, item {}, request {}",
            purchase_event.purchaser, purchase_event.item, purchase_event.request
        );
        purchase_events.send(purchase_event);
    }
    println!();
}

fn should_exit(
    purchase_response_events: EventReader<PurchaseResponse>,
    mut exit: EventWriter<AppExit>,
    mut print_tables: EventWriter<PrintTable>,
    db: Res<AnyDatabaseResource>,
) {
    if purchase_response_events.len() == 2 {
        exit.send(AppExit);
        let request = db.start_new_transaction();
        print_tables.send(PrintTable { request });
    }
}

fn runner(mut app: App) {
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

pub struct MarketplacePlugin;

impl Plugin for MarketplacePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Purchase>()
            .add_event::<Sell>()
            .add_event::<GetSellerItems>()
            .add_event::<PurchaseResponse>()
            .add_event::<PrintTable>()
            .add_systems(Startup, create_tables)
            .add_systems(PostStartup, preload_events)
            .add_systems(PreUpdate, should_exit)
            .add_systems(Update, purchase_system)
            .add_systems(Update, flush_purchase.after(purchase_system))
            .add_systems(PostUpdate, print_items_table)
            .add_systems(PostUpdate, print_users_table)
            .add_systems(PostUpdate, print_purchased_items_table);
    }
}

// macro_rules! make_component_mapper_mapper {
//     ($mappper_name:ident, $($name:ident)+) => {
//         pub struct $mappper_name;
//         impl ComponentMapperMapper for $mappper_name {
//             fn update_or_insert_component(
//                 db_entity: DatabaseEntity,
//                 entity: Entity,
//                 component_id: TypeId,
//                 request: RequestId,
//                 world: &mut World,
//             ) -> Result<(), ()> {
//                 let db = world.get_resource::<AnyDatabaseResource>().unwrap();
//                 get_transaction!(tr, request, db);

//                 let ( $(lower!($name), )+ ) = ( $(TypeId::of::<$name>(), )+ );

//                 match component_id {
//                     $(
//                         lower!($name) => {
//                             let component = world.get::<<ItemQuery as ComponentMapper>::Component>(entity).unwrap();
//                             SingleComponentRetriever::<ItemQuery, AnyDatabaseResource>::update_component(db, world.as_unsafe_world_cell(), &db_entity, component);
//                         }
//                     )+
//                }
//                 Ok(())
//             }
//         }
//     };
// }

// make_component_mapper_mapper!(MarketplaceComponentMapperMapper, ItemQuery);

pub struct MarketplaceComponentMapperMapper;
impl ComponentMapperMapper for MarketplaceComponentMapperMapper {
    fn update_or_insert_component(
        db_entity: DatabaseEntity,
        entity: Entity,
        component_type_id: TypeId,
        component_id: ComponentId,
        request: RequestId,
        world: &mut World,
    ) -> Result<(), ()> {
        let unsafe_world = world.as_unsafe_world_cell();
        let db = unsafe {
            unsafe_world
                .world()
                .get_resource::<AnyDatabaseResource>()
                .unwrap()
        };
        get_transaction!(_tr, request, db);

        if false {
            unreachable!();
        } else if component_type_id == TypeId::of::<MarketItem>() {
            let component: &MarketItem = unsafe {
                unsafe_world
                    .world()
                    .get_by_id(entity, component_id)
                    .unwrap()
                    .deref()
            };

            println!("Updating or inserting MarketItem");
            SingleComponentRetriever::<ItemQuery, AnyDatabaseResource>::update_or_insert_component(
                db,
                unsafe_world,
                &db_entity,
                component,
            )?;
        } else if component_type_id == TypeId::of::<PurchasedItem>() {
            let component: &PurchasedItem = unsafe {
                unsafe_world
                    .world()
                    .get_by_id(entity, component_id)
                    .unwrap()
                    .deref()
            };

            println!("Updating or inserting PurchasedItem");
            SingleComponentRetriever::<PurchaseItemQuery, AnyDatabaseResource>::update_or_insert_component(
                db,
                unsafe_world,
                &db_entity,
                component,
            )?;
        } else if component_type_id == TypeId::of::<User>() {
            let component: &User = unsafe {
                unsafe_world
                    .world()
                    .get_by_id(entity, component_id)
                    .unwrap()
                    .deref()
            };

            println!("Updating or inserting User");
            SingleComponentRetriever::<UserQuery, AnyDatabaseResource>::update_or_insert_component(
                db,
                unsafe_world,
                &db_entity,
                component,
            )?;
        } else if component_type_id == TypeId::of::<Buyer>() {
            let component: &Buyer = unsafe {
                unsafe_world
                    .world()
                    .get_by_id(entity, component_id)
                    .unwrap()
                    .deref()
            };

            println!("Updating or inserting Buyer");
            SingleComponentRetriever::<BuyerQuery, AnyDatabaseResource>::update_or_insert_component(
                db,
                unsafe_world,
                &db_entity,
                component,
            )?;
        } else if component_type_id == TypeId::of::<Seller>() {
            let component: &Seller = unsafe {
                unsafe_world
                    .world()
                    .get_by_id(entity, component_id)
                    .unwrap()
                    .deref()
            };

            println!("Updating or inserting Seller");
            SingleComponentRetriever::<SellerQuery, AnyDatabaseResource>::update_or_insert_component(
                db,
                unsafe_world,
                &db_entity,
                component,
            )?;
        } else {
            return Err(());
        }
        Ok(())
    }
}

// impl_flush_component_to_db!(ItemQuery PurchaseItemQuery UserQuery BuyerQuery SellerQuery);

#[tokio::main]
async fn main() {
    App::new()
        .set_runner(runner)
        .add_plugins(EntityRelationMapperPlugin)
        .add_systems(
            PostUpdate,
            flush_component_to_db::<(
                Option<&UserQuery>,
                Option<&PurchaseItemQuery>,
                Option<&BuyerQuery>,
                Option<&SellerQuery>,
                Option<&ItemQuery>,
            )>,
        )
        .add_plugins(MarketplacePlugin)
        .run();
}
