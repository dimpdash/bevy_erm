mod components;
mod events;
mod queries;
mod resources;
mod systems;

#[macro_use]
extern crate prettytable;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_erm::*;

use events::*;
use queries::*;
use resources::*;
use systems::*;

pub struct MarketplacePlugin;

impl Plugin for MarketplacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EntityRelationMapperPlugin)
            .add_systems(
                PostUpdate,
                flush_component_to_db::<
                    (
                        Option<&UserQuery>,
                        Option<&PurchaseItemQuery>,
                        Option<&BuyerQuery>,
                        Option<&SellerQuery>,
                        Option<&ItemQuery>,
                    ),
                    AnyDatabaseResource,
                >,
            )
            .add_event::<Purchase>()
            .add_event::<Sell>()
            .add_event::<GetSellerItems>()
            .add_event::<PurchaseResponse>()
            .add_event::<PrintTable>()
            .init_resource::<WebServer>()
            .add_systems(Startup, create_tables)
            .add_systems(PreUpdate, poll_webserver_for_requests)
            .add_systems(PreUpdate, should_exit)
            .add_systems(Update, purchase_system)
            .add_systems(Update, flush_purchase.after(purchase_system))
            .add_systems(PostUpdate, print_items_table)
            .add_systems(PostUpdate, print_users_table)
            .add_systems(PostUpdate, print_purchased_items_table);
    }
}

#[tokio::main]
async fn main() {
    App::new()
        .set_runner(runner)
        .add_plugins(MarketplacePlugin)
        .run();
}
