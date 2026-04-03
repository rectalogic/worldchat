use std::str::FromStr;

use super::channel::{Channel, ChannelOfServer};
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
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

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {}
}

struct WsSender(SplitSink<ws::WebSocketStream, ws::Message>);

impl WsSender {
    async fn send(&mut self, command: &Command) -> ws::error::Result<()> {
        self.0.send(ws::Message::text(String::from(command))).await
    }
}

//XXX username needs to come from user at runtime, make this Component and spawn task On<Add, Server>
// XXX match Channel to Server by Entity? pass Server Entity to Channel so we can route messages?
// use Relationship?
// also need User and relationship to Channels they are in - ChannelUsers(Vec<Entity)) and UserOfChannel(Entity)

#[derive(Component, Debug)]
#[relationship_target(relationship = ChannelOfServer, linked_spawn)]
pub struct ServerChannels(Vec<Entity>);

#[derive(Component, Debug)]
pub struct Server {
    task: Task<()>,
    user: String,
}

impl Server {
    pub fn new(server_url: String, user: String) -> Self {
        let u = user.clone();
        let task = IoTaskPool::get().spawn(async move {
            if let Err(e) = Self::serve(server_url, u).await {
                error!("Failed to connect to IRC server: {e:?}");
                //XXX handle ws errors, sleep and retry
            }
        });
        Self { task, user }
    }

    pub(crate) fn join(&mut self, channel: (Entity, String)) {
        //XXX tx a join event
    }

    pub(crate) fn leave(&mut self, channel: (Entity, String)) {
        //XXX tx a leave event
    }

    async fn serve(server_url: String, user: String) -> Result<(), BevyError> {
        let stream = ws::connect_with_protocols(&server_url, &["text.ircv3.net"]).await?;
        let (ws_tx, mut ws_rx) = stream.split();
        let mut ws_tx = WsSender(ws_tx);

        // Send a CAP END to signify that we're IRCv3-compliant (and to end negotiations!).
        ws_tx
            .send(&Command::CAP(None, CapSubCommand::END, None, None))
            .await?;

        ws_tx
            .send(&Command::USER(user.clone(), "0".into(), user.clone()))
            .await?;

        ws_tx.send(&Command::NICK(user)).await?;

        while let Some(response) = ws_rx.next().await {
            if let Ok(ws::Message::Text(bytes)) = response
                && let Ok(message) =
                    irc_proto::message::Message::from_str(bytes.to_string().as_str())
            {
                info!("{message:?}");
                match message.command {
                    Command::PING(server1, server2) => {
                        ws_tx.send(&Command::PONG(server1, server2)).await?;
                    }
                    Command::Response(Response::RPL_ENDOFMOTD, _)
                    | Command::Response(Response::ERR_NOMOTD, _) => {
                        ws_tx
                            .send(&Command::JOIN("#bevyworldchat".into(), None, None))
                            .await?;
                        info!("joined channel");
                    }
                    _ => {}
                }
            }
        }
        //XXX once we get MOTD above, start futures_util::stream::select_all::select_all to listen for join and privmsg messages from bevy

        Ok(())
    }
}
