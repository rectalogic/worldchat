use std::{pin::pin, str::FromStr};

use super::{
    channel::{Channel, ChannelOfServer},
    message::{IrcControl, IrcEvent},
    user::{User, UserOfChannel},
};
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use futures_util::{
    SinkExt, StreamExt,
    stream::{self, SplitSink},
};
use irc_proto::{
    Prefix,
    command::{CapSubCommand, Command},
    message::Message as IrcMessage,
    response::Response,
};
use tokio_tungstenite_wasm as ws;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_add)
            .add_systems(Update, handle_server_events);
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
    rx: async_channel::Receiver<IrcEvent>,
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

    pub(crate) fn send(&mut self, message: IrcControl) -> Result<(), BevyError> {
        if let Some(ServerTask { ref tx, .. }) = self.server_task {
            tx.send_blocking(message)?;
        }
        Ok(())
    }

    fn spawn(&mut self, server_url: String, user: String) {
        let (bevy_tx, bevy_rx) = async_channel::unbounded();
        let (irc_tx, irc_rx) = async_channel::unbounded();
        self.server_task = Some(ServerTask {
            tx: bevy_tx,
            rx: irc_rx,
            _task: IoTaskPool::get().spawn(async move {
                if let Err(e) = Self::serve(server_url, user, bevy_rx, irc_tx).await {
                    error!("Failed to connect to IRC server: {e:?}");
                    //XXX handle ws errors, just alert user?
                }
            }),
        });
    }

    async fn serve(
        server_url: String,
        user: String,
        bevy_rx: async_channel::Receiver<IrcControl>,
        irc_tx: async_channel::Sender<IrcEvent>,
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
                && let Ok(message) = IrcMessage::from_str(bytes.to_string().as_str())
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
        let mut events = pin!(events);
        while let Some(response) = events.as_mut().next().await {
            match response {
                StreamMessage::WsMessage(Ok(ws::Message::Text(bytes))) => {
                    if let Ok(message) =
                        irc_proto::message::Message::from_str(bytes.to_string().as_str())
                    {
                        info!("{message:?}"); //XXX
                        match message {
                            IrcMessage {
                                command: Command::PING(server1, server2),
                                ..
                            } => {
                                ws_tx.send(&Command::PONG(server1, server2)).await?;
                            }
                            //XXX handle the rest
                            IrcMessage {
                                command: Command::JOIN(channel, ..),
                                prefix: Some(Prefix::Nickname(user, ..)),
                                ..
                            } => {
                                irc_tx.send(IrcEvent::UserJoined { channel, user }).await?;
                            }
                            _ => {}
                        }
                    } else {
                        error!("Invalid message {}", bytes.to_string());
                    }
                }
                StreamMessage::WsMessage(Err(e)) => return Err(e.into()),
                StreamMessage::WsMessage(Ok(ws::Message::Binary(_))) => {}
                StreamMessage::WsMessage(Ok(ws::Message::Close(_))) => return Ok(()),
                StreamMessage::IrcControl(control) => match control {
                    IrcControl::Join { channel } => {
                        ws_tx.send(&Command::JOIN(channel, None, None)).await?;
                    }
                    IrcControl::Part { channel } => {
                        ws_tx.send(&Command::PART(channel, None)).await?;
                    }
                    IrcControl::Message { channel, message } => todo!(),
                },
            }
        }
        Ok(())
    }
}

fn on_add(add: On<Add, Server>, mut servers: Query<&mut Server>) {
    if let Ok(mut server) = servers.get_mut(add.entity) {
        let server_url = server.server_url.clone();
        let user = server.user.clone();
        server.spawn(server_url, user);
    }
}

//XXX listen for server events, fire EntityEvents for the corresponding channel
fn handle_server_events(
    servers: Query<&Server>,
    channels: Query<(&Name, &ChannelOfServer), With<Channel>>,
    users: Query<(&Name, &UserOfChannel), With<User>>,
) {
    for server in servers {
        if let Some(ref server_task) = server.server_task {
            while let Ok(event) = server_task.rx.try_recv() {
                match event {
                    //XXX related query
                    IrcEvent::UserJoined { channel, user } => todo!(),
                    IrcEvent::UserParted { channel, user } => todo!(),
                }
            }
        }
    }
}
