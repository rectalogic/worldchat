use bevy::prelude::*;

mod channel;
mod message;
mod server;
mod user;

pub use channel::{ChannelBundle, ChannelOfServer};
pub use server::Server;

pub struct IrcPlugin;

impl Plugin for IrcPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(server::ServerPlugin);
    }
}
