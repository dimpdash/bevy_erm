use async_trait::async_trait;
use bevy_erm::{
    database_query::{CustomDatabaseQuery, DatabaseTransaction},
    *,
};

use crate::components::*;

use sqlx::Row;


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
