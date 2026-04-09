use bevy::prelude::*;

mod channel;
mod message;
mod server;
mod user;

pub use channel::{ChannelOfServer, ChannelUsers, UserAdded, send_channel_message};
pub use message::IrcControl;
pub use server::{Server, ServerChannels, UserNameChanged};
pub use user::{PrimaryUser, UserMessage, UserOfChannel};

pub struct IrcPlugin;

impl Plugin for IrcPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(server::ServerPlugin);
    }
}
