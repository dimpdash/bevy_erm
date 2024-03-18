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
