mod app;
mod irc;
mod world;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

use app::AppPlugin;

#[wasm_bindgen]
pub fn start() {
    App::new().add_plugins(AppPlugin).run();
}
