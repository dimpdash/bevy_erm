use bevy_ecs_macros::Event;
use bevy_erm::*;

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
    pub request: RequestId,
}

#[derive(Event)]
pub struct PrintTable {
    pub request: RequestId,
}