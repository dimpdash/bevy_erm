use bevy_ecs::{component::Component, prelude::*, schedule};
use bevy_erm::*;
use async_trait::async_trait;

use bevy_app::prelude::*;
use futures::executor::block_on;
use sqlx::{FromRow, Row};
use crate::components::*;

pub struct BuyerQuery {}
impl BuyerQuery {
    pub fn load_all() -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, Buyer)>, ()>
    {
        move |conn: &mut sqlx::SqliteConnection| {
            let buyers = block_on(sqlx::query("SELECT id FROM users WHERE buyer = 1").fetch_all(conn)).unwrap();

            let buyers = buyers
                .into_iter()
                .map(|row| {
                    let id = row.get("id");

                    (
                        DatabaseEntity {
                            id,
                            persisted: true.into(),
                            dirty: false,
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
impl DatabaseQueryInfo for BuyerQuery {
    type Component = Buyer;
    type Database = AnyDatabaseResource;
    type Index = DatabaseEntityIndex;

    fn get_component<'c, E>(conn: E, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let buyer_bool = block_on(
            sqlx::query("SELECT buyer FROM users WHERE id = ?")
                .bind(db_entity.id)
                .fetch_one(conn),
        );
        match buyer_bool {
            Ok(_) => Ok(Buyer {}),
            Err(_) => Err(()),
        }
    }

    async fn update_component<'c, E>(
        _tr: E,
        _db_entity: &DatabaseEntity,
        _component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        Ok(())
    }

    async fn insert_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        _component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = block_on(
            sqlx::query("UPDATE users SET buyer = 1 WHERE id = ?")
                .bind(db_entity.id)
                .execute(tr),
        );

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntity) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = block_on(
            sqlx::query("UPDATE users SET buyer = 0 WHERE id = ?")
                .bind(db_entity.id)
                .execute(tr),
        );
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}


pub struct UserQuery {}

impl UserQuery {
    pub fn load_all() -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, User)>, ()>
    {
        move |conn: &mut sqlx::SqliteConnection| {
            let users = block_on(sqlx::query("SELECT id, name FROM users").fetch_all(conn)).unwrap();

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
impl DatabaseQueryInfo for UserQuery {
    type Component = User;
    type Database = AnyDatabaseResource;
    type Index = DatabaseEntityIndex;

    fn get_component<'c, E>(conn: E, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let user = block_on(
            sqlx::query_as::<_, User>("SELECT name FROM users WHERE id = ?")
                .bind(db_entity.id)
                .fetch_one(conn),
        )
        .unwrap();
        Ok(user)
    }

    async fn update_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = sqlx::query("UPDATE users SET name = ? WHERE id = ?")
            .bind(component.name.clone())
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
        let r = sqlx::query("INSERT INTO users (id, name) VALUES (?, ?)")
            .bind(db_entity.id)
            .bind(component.name.clone())
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
        let r = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(db_entity.id)
            .execute(tr)
            .await;
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

pub struct SellerQuery {}

impl SellerQuery {
    pub fn load_all() -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, Seller)>, ()>
    {
        move |conn: &mut sqlx::SqliteConnection| {
            let sellers = block_on(sqlx::query("SELECT id FROM users WHERE seller = 1").fetch_all(conn)).unwrap();

            let sellers = sellers
                .into_iter()
                .map(|row| {
                    let id = row.get("id");

                    (
                        DatabaseEntity {
                            id,
                            persisted: true.into(),
                            dirty: false,
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
impl DatabaseQueryInfo for SellerQuery {
    type Component = Seller;
    type Database = AnyDatabaseResource;
    type Index = DatabaseEntityIndex;

    fn get_component<'c, E>(conn: E, db_entity: &DatabaseEntity) -> Result<Self::Component, ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let seller_bool = block_on(
            sqlx::query("SELECT seller FROM users WHERE id = ?")
                .bind(db_entity.id)
                .fetch_one(conn),
        );
        match seller_bool {
            Ok(_) => Ok(Seller {}),
            Err(_) => Err(()),
        }
    }

    async fn update_component<'c, E>(
        _tr: E,
        _db_entity: &DatabaseEntity,
        _component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        Ok(())
    }

    async fn insert_component<'c, E>(
        tr: E,
        db_entity: &DatabaseEntity,
        _component: &Self::Component,
    ) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = block_on(
            sqlx::query("UPDATE users SET seller = 1 WHERE id = ?")
                .bind(db_entity.id)
                .execute(tr),
        );

        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    async fn delete_component<'c, E>(tr: E, db_entity: &DatabaseEntity) -> Result<(), ()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Sqlite>,
    {
        let r = block_on(
            sqlx::query("UPDATE users SET seller = 0 WHERE id = ?")
                .bind(db_entity.id)
                .execute(tr),
        );
        match r {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

pub struct ItemQuery {}

impl ItemQuery {
    pub fn load_all() -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, MarketItem)>, ()>
    {
        move |conn: &mut sqlx::SqliteConnection| {
            let items = block_on(sqlx::query("SELECT id, seller_id, name, price FROM items").fetch_all(conn)).unwrap();

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

    pub fn load_items_of_seller(
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

pub struct PurchaseItemQuery {}

impl PurchaseItemQuery {
    pub fn load_all() -> impl FnOnce(&mut sqlx::SqliteConnection) -> Result<Vec<(DatabaseEntity, PurchasedItem)>, ()>
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
                        },
                        PurchasedItem {
                            item: DatabaseEntity {
                                id: item,
                                persisted: true.into(),
                                dirty: false,
                            },
                            buyer: DatabaseEntity {
                                id: buyer,
                                persisted: true.into(),
                                dirty: false,
                            },
                        },
                    )
                })
                .collect();

            Ok(items)
        }
    }
}

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