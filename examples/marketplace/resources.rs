use bevy_ecs::prelude::*;
use bevy_erm::*;


use bevy_app::{prelude::*, AppExit};

use crate::components::*;
use futures::executor::block_on;
use crate::queries::*;
use crate::events::*;



#[derive(Resource)]
pub struct WebServer {
    requests_to_send: u32,
}

impl WebServer {
    pub fn should_exit(&self) -> bool {
        self.requests_to_send == 0
    }

    pub fn respond_purchase_event(&mut self, _purchase_response: &PurchaseResponse) {
        println!("Responding to purchase event");
        self.requests_to_send -= 1;
    }

    pub fn is_messages_to_send(&self) -> bool {
        self.requests_to_send > 0
    }
}

impl Default for WebServer {
    fn default() -> Self {
        let requests_to_send = 9;

        println!(
            "Creating WebServer with {} requests to send",
            requests_to_send
        );

        WebServer { requests_to_send }
    }
}
