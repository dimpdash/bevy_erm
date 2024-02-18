use std::borrow::BorrowMut;

use async_trait::async_trait;
use bevy_erm::*;

use crate::components::*;
use futures::executor::block_on;
use sqlx::Row;

pub struct BuyerQuery {}
impl BuyerQuery {
    pub fn load_all(
        request: RequestId,
    ) -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, Buyer)>, ()> {
        move |conn: &mut sqlx::SqliteConnection| {
            let buyers =
                block_on(sqlx::query("SELECT id FROM users WHERE buyer = 1").fetch_all(conn))
                    .unwrap();

            let buyers = buyers
                .into_iter()
                .map(|row| {
                    let id = row.get("id");

                    (
                        DatabaseEntity {
                            id,
                            persisted: true.into(),
                            dirty: false,
                            request,
                        },
                        Buyer {},
                    )
                })
                .collect();

            Ok(buyers)
        }
    }
}

#[async_trait]
impl ComponentMapper for BuyerQuery {
    type Component = Buyer;
    type Executor = <bevy_erm::AnyDatabaseResource as bevy_erm::DatabaseResource>::Transaction;

    async fn get<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
    ) -> Result<Self::Component, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let buyer_bool = sqlx::query("SELECT buyer FROM users WHERE id = ?")
            .bind(db_entity)
            .fetch_one(&mut **tr)
            .await;
        match buyer_bool {
            Ok(_) => Ok(Buyer {}),
            Err(_) => Err(()),
        }
    }

    async fn update_component<'c>(
        _tr: &mut Self::Executor,
        _db_entity: &DatabaseEntityId,
        _component: &Self::Component,
    ) -> Result<(), ()> {
        Ok(())
    }

    async fn insert_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        _component: &Self::Component,
    ) -> Result<(), ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let r = sqlx::query("UPDATE users SET buyer = 1 WHERE id = ?")
            .bind(db_entity)
            .execute(&mut **tr)
            .await;

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    // async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntityId) -> Result<(), ()>
    // where
    //     E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    // {
    //     let r = block_on(
    //         sqlx::query("UPDATE users SET buyer = 0 WHERE id = ?")
    //             .bind(db_entity)
    //             .execute(tr),
    //     );
    //     match r {
    //         Ok(_) => Ok(()),
    //         Err(_) => Err(()),
    //     }
    // }
}

pub struct UserQuery {}

impl UserQuery {
    pub fn load_all(
        request: RequestId,
    ) -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, User)>, ()> {
        move |conn: &mut sqlx::SqliteConnection| {
            let users =
                block_on(sqlx::query("SELECT id, name FROM users").fetch_all(conn)).unwrap();

            let users = users
                .into_iter()
                .map(|row| {
                    let id = row.get("id");
                    let name = row.get("name");

                    (
                        DatabaseEntity {
                            id,
                            persisted: true.into(),
                            dirty: false,
                            request,
                        },
                        User { name },
                    )
                })
                .collect();

            Ok(users)
        }
    }
}

#[async_trait]
impl ComponentMapper for UserQuery {
    type Component = User;
    type Executor = <bevy_erm::AnyDatabaseResource as bevy_erm::DatabaseResource>::Transaction;

    async fn get<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
    ) -> Result<Self::Component, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let user = sqlx::query_as::<_, User>("SELECT name FROM users WHERE id = ?")
            .bind(db_entity)
            .fetch_one(&mut **tr)
            .await
            .unwrap();
        Ok(user)
    }

    async fn update_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        
        let r = sqlx::query("UPDATE users SET name = ? WHERE id = ?")
            .bind(component.name.clone())
            .bind(db_entity)
            .execute(&mut **tr)
            .await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn insert_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let r = sqlx::query("INSERT INTO users (id, name) VALUES (?, ?)")
            .bind(db_entity)
            .bind(component.name.clone())
            .execute(&mut **tr)
            .await;

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    // async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntityId) -> Result<(), ()>
    // where
    //     E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    // {
    //     let r = sqlx::query("DELETE FROM users WHERE id = ?")
    //         .bind(db_entity)
    //         .execute(tr)
    //         .await;
    //     match r {
    //         Ok(_) => Ok(()),
    //         Err(_) => Err(()),
    //     }
    // }
}

pub struct SellerQuery {}

impl SellerQuery {
    pub fn load_all(
        request: RequestId,
    ) -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, Seller)>, ()> {
        move |conn: &mut sqlx::SqliteConnection| {
            let sellers =
                block_on(sqlx::query("SELECT id FROM users WHERE seller = 1").fetch_all(conn))
                    .unwrap();

            let sellers = sellers
                .into_iter()
                .map(|row| {
                    let id = row.get("id");

                    (
                        DatabaseEntity {
                            id,
                            persisted: true.into(),
                            dirty: false,
                            request,
                        },
                        Seller {},
                    )
                })
                .collect();

            Ok(sellers)
        }
    }
}

#[async_trait]
impl ComponentMapper for SellerQuery {
    type Component = Seller;
    type Executor = <bevy_erm::AnyDatabaseResource as bevy_erm::DatabaseResource>::Transaction;

    async fn get<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
    ) -> Result<Self::Component, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        
        let seller_bool = sqlx::query("SELECT seller FROM users WHERE id = ?")
            .bind(db_entity)
            .fetch_one(&mut **tr)
            .await;
        match seller_bool {
            Ok(_) => Ok(Seller {}),
            Err(_) => Err(()),
        }
    }

    async fn update_component<'c>(
        _tr: &mut Self::Executor,
        _db_entity: &DatabaseEntityId,
        _component: &Self::Component,
    ) -> Result<(), ()> {
        Ok(())
    }

    async fn insert_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        _component: &Self::Component,
    ) -> Result<(), ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        
        let r = sqlx::query("UPDATE users SET seller = 1 WHERE id = ?")
            .bind(db_entity)
            .execute(&mut **tr)
            .await;

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    // async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntityId) -> Result<(), ()>
    // where
    //     E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    // {
    //     let r = block_on(
    //         sqlx::query("UPDATE users SET seller = 0 WHERE id = ?")
    //             .bind(db_entity)
    //             .execute(tr),
    //     );
    //     match r {
    //         Ok(_) => Ok(()),
    //         Err(_) => Err(()),
    //     }
    // }
}

pub struct ItemQuery {}

impl ItemQuery {
    pub fn load_all(
        request: RequestId,
    ) -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, MarketItem)>, ()>
    {
        move |conn: &mut sqlx::SqliteConnection| {
            let items = block_on(
                sqlx::query("SELECT id, seller_id, name, price FROM items").fetch_all(conn),
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
                            request,
                        },
                        MarketItem {
                            seller_id,
                            name,
                            price,
                        },
                    )
                })
                .collect();

            Ok(items)
        }
    }

    #[allow(dead_code)]
    pub fn load_items_of_seller(
        seller: DatabaseEntity,
    ) -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, MarketItem)>, ()>
    {
        move |conn: &mut sqlx::SqliteConnection| {
            let items = block_on(
                sqlx::query("SELECT id, seller_id, name, price FROM items WHERE seller_id = ?")
                    .bind(seller.id.0)
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
                            request: seller.request,
                        },
                        MarketItem {
                            seller_id,
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
impl ComponentMapper for ItemQuery {
    type Component = MarketItem;
    type Executor = <bevy_erm::AnyDatabaseResource as bevy_erm::DatabaseResource>::Transaction;

    async fn get<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
    ) -> Result<Self::Component, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        
        let item = sqlx::query_as::<_, MarketItem>("SELECT * FROM items WHERE id = ?")
            .bind(db_entity)
            .fetch_one(&mut **tr)
            .await
            .unwrap();
        Ok(item)
    }

    async fn update_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let r = sqlx::query("UPDATE items SET seller_id = ?, name = ?, price = ? WHERE id = ?")
            .bind(component.seller_id)
            .bind(component.name.clone())
            .bind(component.price)
            .bind(db_entity)
            .execute(&mut **tr)
            .await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn insert_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let r = sqlx::query("INSERT INTO items (id, seller_id, name, price) VALUES (?, ?, ?, ?)")
            .bind(db_entity)
            .bind(component.seller_id)
            .bind(component.name.clone())
            .bind(component.price)
            .execute(&mut **tr)
            .await;

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    // async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntityId) -> Result<(), ()>
    // where
    //     E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    // {
    //     let r = sqlx::query("DELETE FROM items WHERE id = ?")
    //         .bind(db_entity)
    //         .execute(tr)
    //         .await;
    //     match r {
    //         Ok(_) => Ok(()),
    //         Err(_) => Err(()),
    //     }
    // }
}

pub struct PurchaseItemQuery {}

impl PurchaseItemQuery {
    pub fn load_all(
        request: RequestId,
    ) -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, PurchasedItem)>, ()>
    {
        move |conn: &mut sqlx::SqliteConnection| {
            let items = block_on(
                sqlx::query("SELECT id, item, buyer FROM purchased_items").fetch_all(conn),
            )
            .unwrap();

            let items = items
                .into_iter()
                .map(|row| {
                    let id = row.get("id");
                    let item = row.get("item");
                    let buyer = row.get("buyer");

                    (
                        DatabaseEntity {
                            id,
                            persisted: true.into(),
                            dirty: false,
                            request,
                        },
                        PurchasedItem { item, buyer },
                    )
                })
                .collect();

            Ok(items)
        }
    }
}

#[async_trait]
impl ComponentMapper for PurchaseItemQuery {
    type Executor = <bevy_erm::AnyDatabaseResource as bevy_erm::DatabaseResource>::Transaction;
    type Component = PurchasedItem;

    async fn get<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
    ) -> Result<Self::Component, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        
        let item =
            sqlx::query_as::<_, PurchasedItem>("SELECT item FROM purchased_items WHERE id = ?")
                .bind(db_entity)
                .fetch_one(&mut **tr)
                .await
                .unwrap();
        Ok(item)
    }

    async fn update_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let r = sqlx::query("UPDATE purchased_items SET item = ?, buyer = ? WHERE id = ?")
            .bind(component.item)
            .bind(component.buyer)
            .bind(db_entity)
            .execute(&mut **tr)
            .await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn insert_component<'c>(
        tr: &mut Self::Executor,
        db_entity: &DatabaseEntityId,
        component: &Self::Component,
    ) -> Result<(), ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let r = sqlx::query("INSERT INTO purchased_items (id, item, buyer) VALUES (?, ?, ?)")
            .bind(db_entity.0)
            .bind(component.item)
            .bind(component.buyer)
            .execute(&mut **tr)
            .await;

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    // async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntityId) -> Result<(), ()>
    // where
    //     E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    // {
    //     let r = sqlx::query("DELETE FROM purchased_items WHERE id = ?")
    //         .bind(db_entity)
    //         .execute(tr)
    //         .await;
    //     match r {
    //         Ok(_) => Ok(()),
    //         Err(_) => Err(()),
    //     }
    // }
}
