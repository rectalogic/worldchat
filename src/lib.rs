mod app;
mod irc;
mod world;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use app::{AppPlugin, send_message};
pub use irc::UserMessage;

#[wasm_bindgen]
pub fn start(user_name: String) {
    App::new().add_plugins(AppPlugin { user_name }).run();
}

#[wasm_bindgen]
pub fn message(message: String) {
    if let Err(e) = send_message(message) {
        error!("Failed to send message: {e:?}");
    }
}
