use std::{pin::pin, str::FromStr};

use super::{
    message::{IrcControlMessage, IrcEvent},
    user::{PrimaryUser, UserJoined, UserMessage},
};
use bevy::{
    platform::collections::HashMap,
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use futures_util::{
    SinkExt, StreamExt,
    stream::{self, SplitSink},
};
use irc_proto::{
    ChannelExt, Prefix,
    command::{CapSubCommand, Command},
    message::Message as IrcMessage,
    response::Response,
};
use tokio_tungstenite_wasm as ws;

pub struct IrcServerPlugin {
    pub server_url: String,
    pub channel: String,
    pub user: String,
}

impl Plugin for IrcServerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(IrcServer::new(
            self.server_url.clone(),
            self.channel.clone(),
            self.user.clone(),
        ))
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
    IrcControl(IrcControlMessage),
    WsMessage(ws::error::Result<ws::Message>),
}

#[derive(Resource, Debug)]
pub struct IrcServer {
    tx: async_channel::Sender<IrcControlMessage>,
    rx: async_channel::Receiver<IrcEvent>,
    _task: Task<()>,
    // Map nick to entity
    users: HashMap<String, Entity>,
}

impl IrcServer {
    pub fn new(server_url: String, channel: String, user: String) -> Self {
        let (bevy_tx, bevy_rx) = async_channel::unbounded();
        let (irc_tx, irc_rx) = async_channel::unbounded();
        Self {
            tx: bevy_tx,
            rx: irc_rx,
            _task: IoTaskPool::get().spawn(async move {
                if let Err(e) = Self::serve(server_url, user, channel, bevy_rx, irc_tx).await {
                    error!("Failed to connect to IRC server: {e:?}");
                    //XXX handle ws errors, just alert user?
                }
            }),
            users: HashMap::default(),
        }
    }

    pub fn send(&self, message: IrcControlMessage) -> Result<(), BevyError> {
        self.tx.try_send(message)?;
        Ok(())
    }

    async fn serve(
        server_url: String,
        user: String,
        channel: String,
        bevy_rx: async_channel::Receiver<IrcControlMessage>,
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

        ws_tx.send(&Command::NICK(user.clone())).await?;

        let mut server_nick = user.clone();

        while let Some(response) = ws_rx.next().await {
            if let Ok(ws::Message::Text(bytes)) = response
                && let Ok(message) = IrcMessage::from_str(bytes.to_string().as_str())
            {
                info!("{message:?}"); //XXX
                match message.command {
                    Command::PING(server1, server2) => {
                        ws_tx.send(&Command::PONG(server1, server2)).await?;
                    }
                    Command::Response(Response::ERR_NICKNAMEINUSE, _) => {
                        server_nick.push('_');
                        ws_tx.send(&Command::NICK(server_nick.clone())).await?;
                    }
                    Command::Response(Response::RPL_WELCOME, _) => {
                        ws_tx
                            .send(&Command::JOIN(channel.clone(), None, None))
                            .await?;
                        break;
                    }
                    _ => {}
                }
            }
        }

        irc_tx
            .send(IrcEvent::PrimaryUser {
                nick: server_nick.clone(),
                name: user,
            })
            .await?;

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
                            IrcMessage {
                                command: Command::JOIN(..),
                                prefix: Some(Prefix::Nickname(..)),
                                ..
                            } => {
                                irc_tx.send(IrcEvent::UserJoined).await?;
                            }
                            IrcMessage {
                                command: Command::PART(..),
                                prefix: Some(Prefix::Nickname(nick, ..)),
                                ..
                            } => {
                                irc_tx.send(IrcEvent::Part { nick }).await?;
                            }
                            IrcMessage {
                                command: Command::QUIT(_),
                                prefix: Some(Prefix::Nickname(nick, ..)),
                                ..
                            } => {
                                irc_tx.send(IrcEvent::Quit { nick }).await?;
                            }
                            IrcMessage {
                                command: Command::NICK(nick),
                                prefix: Some(Prefix::Nickname(previous_nick, ..)),
                                ..
                            } => {
                                irc_tx
                                    .send(IrcEvent::ChangeName {
                                        previous_nick,
                                        nick,
                                    })
                                    .await?;
                            }
                            IrcMessage {
                                command: Command::PRIVMSG(channel, message),
                                prefix: Some(Prefix::Nickname(nick, ..)),
                                ..
                            } if channel.is_channel_name() => {
                                irc_tx.send(IrcEvent::Message { nick, message }).await?;
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
                StreamMessage::IrcControl(control) => {
                    info!("{control:?}"); //XXX
                    match control {
                        IrcControlMessage::Part => {
                            ws_tx.send(&Command::PART(channel.clone(), None)).await?;
                        }
                        IrcControlMessage::Message { message } => {
                            ws_tx
                                .send(&Command::PRIVMSG(channel.clone(), message))
                                .await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn handle_server_events(mut commands: Commands, mut server: ResMut<IrcServer>) {
    while let Ok(event) = server.rx.try_recv() {
        match event {
            IrcEvent::PrimaryUser { nick, name } => {
                let entity = commands.spawn((PrimaryUser, Name::new(name))).id();
                server.users.insert(nick, entity);
            }
            IrcEvent::ChangeName {
                previous_nick,
                nick,
            } => {
                if let Some(entity) = server.users.remove(&previous_nick) {
                    server.users.insert(nick, entity);
                }
            }
            IrcEvent::UserJoined => {
                commands.trigger(UserJoined);
            }
            IrcEvent::Part { nick } | IrcEvent::Quit { nick } => {
                if let Some(&user_entity) = server.users.get(&nick) {
                    commands.entity(user_entity).despawn();
                }
            }
            IrcEvent::Message { nick, message } => {
                if let Some(&user_entity) = server.users.get(&nick) {
                    commands.trigger(UserMessage {
                        user_entity,
                        message,
                    });
                }
            }
        };
    }
}
