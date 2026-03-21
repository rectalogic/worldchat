use bevy::prelude::*;
use worldchat::AppPlugin;

#[tokio::main]
async fn main() {
    App::new().add_plugins(AppPlugin).run();
}
