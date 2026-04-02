use bevy::{prelude::*, tasks::IoTaskPool};
use futures_util::{
    Sink, SinkExt, Stream, StreamExt,
    stream::{SplitSink, SplitStream},
};
use irc_proto::{
    command::{CapSubCommand, Command},
    response::Response,
};
use tokio_tungstenite_wasm as ws;
use wasm_bindgen::prelude::*;

mod channel;
mod server;

pub struct IrcPlugin {
    pub server: String,
    pub user: String,
}

impl Plugin for IrcPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(server::ServerPlugin {
            server: self.server.clone(),
            user: self.user.clone(),
        });
        //XXX Server should be a resource, it can send messages per channel subscription?
        // futures_util::stream::select_all::select_all to process a bunch of channels plus the server multiplexed messages
        //
        // need tx/rx per channel to talk to Bevy
        // need control tx/rx to join new channels
        // OR just multiplex bevy messages on single tx/rx - include channel name in message (or Entity holding the Channel component)
    }
}
