use bevy_ecs::component::Component;
use bevy_erm::*;
use sqlx::{FromRow, Row};

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
pub struct User {
    #[allow(dead_code)]
    pub name: String,
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
pub struct Seller {}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow)]
pub struct Buyer {}

#[derive(Component, Debug, Default, Clone)]
pub struct MarketItem {
    pub seller_id: DatabaseEntity,
    pub name: String,
    pub price: i32,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for MarketItem {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(MarketItem {
            seller_id: DatabaseEntity {
                id: row.try_get("seller_id")?,
                persisted: true.into(),
                dirty: false,
            },
            name: row.get("name"),
            price: row.get("price"),
        })
    }
}

#[derive(Component, Debug, Default, Clone)]
pub struct PurchasedItem {
    pub item: DatabaseEntity,
    pub buyer: DatabaseEntity,
}

impl FromRow<'_, sqlx::sqlite::SqliteRow> for PurchasedItem {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(PurchasedItem {
            item: DatabaseEntity {
                id: row.try_get("item")?,
                persisted: true.into(),
                dirty: false,
            },
            buyer: DatabaseEntity {
                id: row.try_get("buyer")?,
                persisted: true.into(),
                dirty: false,
            },
        })
    }
}
