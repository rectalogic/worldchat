use std::{collections::HashMap, pin::pin, str::FromStr};

use super::{
    channel::{Channel, ChannelOfServer},
    message::{IrcControl, IrcResponse},
};
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use futures_util::{
    Sink, SinkExt, Stream, StreamExt,
    stream::{self, SplitSink, SplitStream},
};
use irc_proto::{
    command::{CapSubCommand, Command},
    response::Response,
};
use tokio_tungstenite_wasm as ws;
use wasm_bindgen::prelude::*;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_add);
    }
}

struct WsSender(SplitSink<ws::WebSocketStream, ws::Message>);

impl WsSender {
    async fn send(&mut self, command: &Command) -> ws::error::Result<()> {
        self.0.send(ws::Message::text(String::from(command))).await
    }
}

enum StreamMessage {
    IrcControl(IrcControl),
    WsMessage(ws::error::Result<ws::Message>),
}

//XXX username needs to come from user at runtime, make this Component and spawn task On<Add, Server>
// XXX match Channel to Server by Entity? pass Server Entity to Channel so we can route messages?
// use Relationship?
// also need User and relationship to Channels they are in - ChannelUsers(Vec<Entity)) and UserOfChannel(Entity)

#[derive(Component, Debug)]
#[relationship_target(relationship = ChannelOfServer, linked_spawn)]
pub struct ServerChannels(Vec<Entity>);

#[derive(Debug)]
struct ServerTask {
    tx: async_channel::Sender<IrcControl>,
    rx: async_channel::Receiver<IrcResponse>,
    channels: HashMap<String, Entity>,
    _task: Task<()>,
}

#[derive(Component, Debug)]
pub struct Server {
    server_task: Option<ServerTask>,
    server_url: String,
    user: String,
}

impl Server {
    pub fn new(server_url: String, user: String) -> Self {
        Self {
            server_task: None,
            server_url,
            user,
        }
    }

    pub(crate) fn join(&mut self, channel: String, entity: Entity) -> Result<(), BevyError> {
        if let Some(ServerTask {
            ref tx,
            ref mut channels,
            ..
        }) = self.server_task
        {
            tx.send_blocking(IrcControl::Join {
                channel: channel.clone(),
            })?;
            channels.insert(channel, entity);
        }
        Ok(())
    }

    pub(crate) fn leave(&mut self, channel: String) -> Result<(), BevyError> {
        if let Some(ServerTask {
            ref tx,
            ref mut channels,
            ..
        }) = self.server_task
        {
            channels.remove(&channel);
            tx.send_blocking(IrcControl::Leave { channel })?;
        }
        Ok(())
    }

    fn spawn(&mut self, server_url: String, user: String) {
        let (bevy_tx, bevy_rx) = async_channel::unbounded();
        let (irc_tx, irc_rx) = async_channel::unbounded();
        self.server_task = Some(ServerTask {
            tx: bevy_tx,
            rx: irc_rx,
            channels: HashMap::default(),
            _task: IoTaskPool::get().spawn(async move {
                if let Err(e) = Self::serve(server_url, user, bevy_rx, irc_tx).await {
                    error!("Failed to connect to IRC server: {e:?}");
                    //XXX handle ws errors, sleep and retry
                }
            }),
        });
    }

    async fn serve(
        server_url: String,
        user: String,
        bevy_rx: async_channel::Receiver<IrcControl>,
        irc_tx: async_channel::Sender<IrcResponse>,
    ) -> Result<(), BevyError> {
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
                info!("{message:?}"); //XXX
                match message.command {
                    Command::PING(server1, server2) => {
                        ws_tx.send(&Command::PONG(server1, server2)).await?;
                    }
                    Command::Response(Response::RPL_ENDOFMOTD, _)
                    | Command::Response(Response::ERR_NOMOTD, _) => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        let events = stream::select(
            ws_rx.map(StreamMessage::WsMessage),
            bevy_rx.map(StreamMessage::IrcControl),
        );
        pin!(events);
        while let Some(response) = events.next().await {
            match response {
                StreamMessage::WsMessage(Ok(ws::Message::Text(bytes))) => {
                    if let Ok(message) =
                        irc_proto::message::Message::from_str(bytes.to_string().as_str())
                    {
                        match message.command {
                            Command::PING(server1, server2) => {
                                ws_tx.send(&Command::PONG(server1, server2)).await?;
                            }
                            //XXX handle the rest
                            _ => {}
                        }
                    } else {
                        error!("Invalid message {}", bytes.to_string());
                    }
                }
                StreamMessage::WsMessage(Err(e)) => {}
                StreamMessage::WsMessage(Ok(ws::Message::Binary(_))) => {}
                StreamMessage::WsMessage(Ok(ws::Message::Close(_))) => return Ok(()),
                StreamMessage::IrcControl(control) => match control {
                    IrcControl::Join { channel } => {
                        ws_tx.send(&Command::JOIN(channel, None, None)).await?;
                    }
                    IrcControl::Leave { channel } => todo!(),
                    IrcControl::Message { channel, message } => todo!(),
                },
            }
        }
        Ok(())
    }
}

fn on_add(add: On<Add, Server>, mut query: Query<&mut Server>) {
    if let Ok(mut server) = query.get_mut(add.entity) {
        let server_url = server.server_url.clone();
        let user = server.user.clone();
        server.spawn(server_url, user);
    }
}
