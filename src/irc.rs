use bevy::prelude::*;

mod channel;
mod message;
mod server;
mod user;

pub use channel::ChannelOfServer;
pub use message::IrcControl;
pub use server::Server;
pub use user::{UserMessage, UserOfChannel};

pub struct IrcPlugin;

impl Plugin for IrcPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(server::ServerPlugin);
    }
}
