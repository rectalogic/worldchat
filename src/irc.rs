use bevy::prelude::*;

mod channel;
mod message;
mod server;
mod user;

pub use channel::{Channel, ChannelOfServer};
pub use server::{Server, ServerChannels};

pub struct IrcPlugin;

impl Plugin for IrcPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(server::ServerPlugin);
        //XXX Server should be a resource, it can send messages per channel subscription?
        // futures_util::stream::select_all::select_all to process a bunch of channels plus the server multiplexed messages
        //
        // need tx/rx per channel to talk to Bevy
        // need control tx/rx to join new channels
        // OR just multiplex bevy messages on single tx/rx - include channel name in message (or Entity holding the Channel component)
    }
}
