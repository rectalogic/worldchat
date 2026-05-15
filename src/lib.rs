mod app;
mod irc;
mod world;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use app::{AppPlugin, send_message};
pub use irc::{User, UserMessage};

#[wasm_bindgen]
pub fn start() {
    App::new().add_plugins(AppPlugin).run();
}

#[wasm_bindgen]
pub fn message(message: String) {
    if let Err(e) = send_message(message) {
        error!("Failed to send message: {e:?}");
    }
}
