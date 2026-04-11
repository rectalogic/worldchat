use std::{pin::pin, str::FromStr};

use super::{
    channel::{ChannelOfServer, ChannelPlugin, ChannelUsers, UserAdded},
    find_relationship_source_named,
    message::{IrcControl, IrcEvent},
    user::{UserMessage, UserOfChannel},
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
    ChannelExt, Prefix,
    command::{CapSubCommand, Command},
    message::Message as IrcMessage,
    response::Response,
};
use tokio_tungstenite_wasm as ws;

pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ChannelPlugin)
            .add_observer(on_add)
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

#[derive(Component, Debug)]
#[relationship_target(relationship = ChannelOfServer, linked_spawn)]
pub struct ServerChannels(Vec<Entity>);

#[derive(Debug)]
struct ServerTask {
    tx: async_channel::Sender<IrcControl>,
    rx: async_channel::Receiver<IrcEvent>,
    _task: Task<()>,
}

#[derive(EntityEvent, Debug)]
pub struct UserNameChanged {
    #[event_target]
    pub server_entity: Entity,
    pub previous_name: Name,
    pub name: Name,
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

    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    pub fn irc_tx(&self) -> Option<&async_channel::Sender<IrcControl>> {
        if let Some(ServerTask { ref tx, .. }) = self.server_task {
            Some(tx)
        } else {
            None
        }
    }

    pub fn send(&self, message: IrcControl) -> Result<(), BevyError> {
        if let Some(tx) = self.irc_tx() {
            tx.try_send(message)?;
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
        server_user: String,
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
            .send(&Command::USER(
                server_user.clone(),
                "0".into(),
                server_user.clone(),
            ))
            .await?;

        ws_tx.send(&Command::NICK(server_user.clone())).await?;

        let mut server_user = server_user;

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
                        server_user.push('_');
                        ws_tx.send(&Command::NICK(server_user.clone())).await?;
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
                            IrcMessage {
                                command: Command::Response(Response::RPL_NAMREPLY, ref args),
                                ..
                            } if args.len() == 4 => {
                                let channel = &args[2];
                                for user in args[3].split(' ') {
                                    irc_tx
                                        .send(IrcEvent::AddUser {
                                            channel: channel.clone(),
                                            primary: user == server_user,
                                            joined: false,
                                            user: user.into(),
                                        })
                                        .await?;
                                }
                            }
                            IrcMessage {
                                command: Command::JOIN(channel, ..),
                                prefix: Some(Prefix::Nickname(user, ..)),
                                ..
                            } => {
                                irc_tx
                                    .send(IrcEvent::AddUser {
                                        channel,
                                        primary: user == server_user,
                                        joined: true,
                                        user,
                                    })
                                    .await?;
                            }
                            IrcMessage {
                                command: Command::PART(channel, ..),
                                prefix: Some(Prefix::Nickname(user, ..)),
                                ..
                            } => {
                                irc_tx.send(IrcEvent::Part { channel, user }).await?;
                            }
                            IrcMessage {
                                command: Command::QUIT(_),
                                prefix: Some(Prefix::Nickname(user, ..)),
                                ..
                            } => {
                                irc_tx.send(IrcEvent::Quit { user }).await?;
                            }
                            IrcMessage {
                                command: Command::NICK(name),
                                prefix: Some(Prefix::Nickname(previous_name, ..)),
                                ..
                            } => {
                                irc_tx
                                    .send(IrcEvent::ChangeName {
                                        previous_name,
                                        name,
                                    })
                                    .await?;
                            }
                            IrcMessage {
                                command: Command::PRIVMSG(channel, message),
                                prefix: Some(Prefix::Nickname(user, ..)),
                                ..
                            } if channel.is_channel_name() => {
                                irc_tx
                                    .send(IrcEvent::Message {
                                        channel,
                                        user,
                                        message,
                                    })
                                    .await?;
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
                        IrcControl::Join { channel } => {
                            ws_tx.send(&Command::JOIN(channel, None, None)).await?;
                        }
                        IrcControl::Part { channel } => {
                            ws_tx.send(&Command::PART(channel, None)).await?;
                        }
                        IrcControl::Message { channel, message } => {
                            ws_tx.send(&Command::PRIVMSG(channel, message)).await?;
                        }
                    }
                }
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

fn handle_server_events(
    mut commands: Commands,
    servers: Query<(Entity, &Server)>,
    server_channels: Query<&ServerChannels>,
    channels: Query<(Entity, &Name), With<ChannelOfServer>>,
    channel_users: Query<&ChannelUsers>,
    users: Query<(Entity, &Name), With<UserOfChannel>>,
) {
    for (server_entity, server) in servers {
        if let Some(ref server_task) = server.server_task {
            while let Ok(event) = server_task.rx.try_recv() {
                match event {
                    IrcEvent::ChangeName {
                        previous_name,
                        name,
                    } => {
                        commands.trigger(UserNameChanged {
                            server_entity,
                            previous_name: Name::new(previous_name),
                            name: Name::new(name),
                        });
                    }
                    IrcEvent::AddUser {
                        channel,
                        user,
                        primary,
                        joined,
                    } => {
                        let channel_name = Name::new(channel);
                        if let Some(channel_entity) = find_relationship_source_named(
                            &channel_name,
                            server_entity,
                            server_channels,
                            channels,
                        ) {
                            commands.trigger(UserAdded {
                                server_entity,
                                channel_entity,
                                channel_name,
                                user_name: Name::new(user),
                                primary,
                                joined,
                            });
                        }
                    }
                    IrcEvent::Part { channel, user } => {
                        if let (_, Some(user_entity)) = find_channel_user(
                            &Name::new(channel),
                            &Name::new(user),
                            server_entity,
                            server_channels,
                            channels,
                            channel_users,
                            users,
                        ) {
                            commands.entity(user_entity).despawn();
                        }
                    }
                    IrcEvent::Quit { user } => {
                        let user_name = Name::new(user);
                        // Despawn user in all channels
                        users
                            .iter()
                            .filter(|&(_, user)| *user == user_name)
                            .for_each(|(user_entity, _)| commands.entity(user_entity).despawn());
                    }
                    IrcEvent::Message {
                        channel,
                        user,
                        message,
                    } => {
                        let user_name = Name::new(user);
                        if let (Some(_), Some(user_entity)) = find_channel_user(
                            &Name::new(channel),
                            &user_name,
                            server_entity,
                            server_channels,
                            channels,
                            channel_users,
                            users,
                        ) {
                            commands.trigger(UserMessage {
                                user_entity,
                                message,
                            });
                        }
                    }
                };
            }
        }
    }
}

fn find_channel_user(
    channel: &Name,
    user: &Name,
    server_entity: Entity,
    server_channels: Query<&ServerChannels>,
    channels: Query<(Entity, &Name), With<ChannelOfServer>>,
    channel_users: Query<&ChannelUsers>,
    users: Query<(Entity, &Name), With<UserOfChannel>>,
) -> (Option<Entity>, Option<Entity>) {
    if let Some(channel_entity) =
        find_relationship_source_named(channel, server_entity, server_channels, channels)
    {
        if let Some(user_entity) =
            find_relationship_source_named(user, channel_entity, channel_users, users)
        {
            (Some(channel_entity), Some(user_entity))
        } else {
            (Some(channel_entity), None)
        }
    } else {
        (None, None)
    }
}
