use bevy_ecs::component::Component;
use bevy_erm::*;
use bevy_erm_derive::DBQueryDerive;
use sqlx::{FromRow, Row};
use tokio::main;
use async_trait::async_trait;
use sqlx;

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
pub struct User {
    #[allow(dead_code)]
    pub name: String,
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
pub struct Seller {}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
pub struct Buyer {}

#[derive(Component, Debug, Default, Clone, DBQueryDerive)]
#[table_name = "items"]
pub struct MarketItem {
    pub seller_id: DatabaseEntityId,
    pub name: String,
    pub price: i32,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for MarketItem {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(MarketItem {
            seller_id: row.try_get("seller_id")?,
            name: row.get("name"),
            price: row.get("price"),
        })
    }
}

#[derive(Component, Debug, Default, Clone, DBQueryDerive)]
#[table_name = "purchased_items"]
pub struct PurchasedItem {
    pub item: DatabaseEntityId,
    pub buyer: DatabaseEntityId,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for PurchasedItem {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(PurchasedItem {
            item: row.try_get("item")?,
            buyer: row.try_get("buyer")?,
        })
    }
}
