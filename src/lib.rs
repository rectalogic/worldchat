mod app;
mod irc;
mod world;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use app::AppPlugin;
pub use irc::UserMessage;

#[wasm_bindgen]
pub fn start(user_name: String) {
    App::new().add_plugins(AppPlugin { user_name }).run();
}
