use bevy_ecs::component::Component;
use bevy_erm::*;
use bevy_erm_derive::DBQueryDerive;
use async_trait::async_trait;
use sqlx;
use bevy_erm_core::database_query::{CustomDatabaseQuery, DatabaseTransaction};

#[derive(Component, Debug, Default, Clone, sqlx::FromRow, DBQueryDerive)]
#[table_name = "users"]
pub struct User {
    #[allow(dead_code)]
    pub name: String,
}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow, DBQueryDerive)]
#[table_name = "users"]
pub struct Seller {}

#[derive(Component, Debug, Default, Clone, sqlx::FromRow, DBQueryDerive)]
#[table_name = "users"]
pub struct Buyer {}

#[derive(Component, Debug, Default, Clone, DBQueryDerive, sqlx::FromRow)]
#[table_name = "items"]
pub struct MarketItem {
    pub seller_id: DatabaseEntityId,
    pub name: String,
    pub price: i32,
}

#[derive(Component, Debug, Default, Clone, DBQueryDerive, sqlx::FromRow)]
#[table_name = "purchased_items"]
pub struct PurchasedItem {
    pub item: DatabaseEntityId,
    pub buyer: DatabaseEntityId,
}