use bevy::prelude::*;
use worldchat::AppPlugin;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    App::new().add_plugins(AppPlugin).run();
}
