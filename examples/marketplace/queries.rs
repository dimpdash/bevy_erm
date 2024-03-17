use async_trait::async_trait;
use bevy_erm::{
    database_query::{CustomDatabaseQuery, DatabaseTransaction},
    *,
};

use crate::components::*;

use sqlx::Row;

pub struct BuyerQuery {}

pub struct BuyerQueryLoadAll(pub RequestId);

#[async_trait]
impl CustomDatabaseQuery<AnyDatabaseResource, Buyer> for BuyerQueryLoadAll {
    async fn query(
        &self,
        tr: DatabaseTransaction<AnyDatabaseResource>,
    ) -> Result<Vec<(DatabaseEntity, Buyer)>, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let buyers = sqlx::query("SELECT id FROM users WHERE buyer = 1")
            .fetch_all(&mut **tr)
            .await
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
                        request: self.0,
                    },
                    Buyer {},
                )
            })
            .collect();

        Ok(buyers)
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

pub struct UserQueryLoadAll(pub RequestId);

#[async_trait]
impl CustomDatabaseQuery<AnyDatabaseResource, User> for UserQueryLoadAll {
    async fn query(
        &self,
        tr: DatabaseTransaction<AnyDatabaseResource>,
    ) -> Result<Vec<(DatabaseEntity, User)>, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let users = sqlx::query("SELECT id, name FROM users")
            .fetch_all(&mut **tr)
            .await
            .unwrap();

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
                        request: self.0,
                    },
                    User { name },
                )
            })
            .collect();

        Ok(users)
    }
}
pub struct UserQuery {}

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

pub struct SellerQueryLoadAll(pub RequestId);
#[async_trait]
impl CustomDatabaseQuery<AnyDatabaseResource, Seller> for SellerQueryLoadAll {
    async fn query(
        &self,
        tr: DatabaseTransaction<AnyDatabaseResource>,
    ) -> Result<Vec<(DatabaseEntity, Seller)>, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let sellers = sqlx::query("SELECT id FROM users WHERE seller = 1")
            .fetch_all(&mut **tr)
            .await
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
                        request: self.0,
                    },
                    Seller {},
                )
            })
            .collect();

        Ok(sellers)
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

pub struct ItemQueryLoadAll(pub RequestId);
#[async_trait]
impl CustomDatabaseQuery<AnyDatabaseResource, MarketItem> for ItemQueryLoadAll {
    async fn query(
        &self,
        tr: DatabaseTransaction<AnyDatabaseResource>,
    ) -> Result<Vec<(DatabaseEntity, MarketItem)>, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let items = sqlx::query("SELECT id, seller_id, name, price FROM items")
            .fetch_all(&mut **tr)
            .await
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
                        request: self.0,
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



pub struct PurchaseItemQuery {}

pub struct PurchaseItemQueryLoadAll(pub RequestId);
#[async_trait]
impl CustomDatabaseQuery<AnyDatabaseResource, PurchasedItem> for PurchaseItemQueryLoadAll {
    async fn query(
        &self,
        tr: DatabaseTransaction<AnyDatabaseResource>,
    ) -> Result<Vec<(DatabaseEntity, PurchasedItem)>, ()> {
        let mut guard = tr.lock().await;
        let tr = guard.a.as_mut().unwrap();
        let items = sqlx::query("SELECT id, item, buyer FROM purchased_items")
            .fetch_all(&mut **tr)
            .await
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
                        request: self.0,
                    },
                    PurchasedItem { item, buyer },
                )
            })
            .collect();

        Ok(items)
    }
}
