pub use crate::app::AppPlugin;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn start() {
    App::new().add_plugins(AppPlugin).run();
}
