use std::{pin::pin, str::FromStr};

use crate::{app::AppState, irc::user::PrimaryUserNameSet};

use super::{
    message::IrcControlMessage,
    user::{PrimaryUser, User, UserJoined, UserMessage},
};
use bevy::{
    platform::collections::HashMap,
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use futures_util::{
    SinkExt, Stream, StreamExt,
    stream::{self, SplitSink},
};
use irc_proto::{
    ChannelExt, Prefix,
    command::{CapSubCommand, Command},
    message::Message as IrcMessage,
    response::Response,
};
use tokio_tungstenite_wasm as ws;

const IRC_SERVER_URL: &str = "wss://fiery.swiftirc.net:4443";
const IRC_CHANNEL: &str = "#bevyworldchat";

pub struct IrcServerPlugin;

impl Plugin for IrcServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_username_set)
            .add_systems(OnExit(AppState::Chat), teardown)
            .add_systems(
                Update,
                handle_server_events.run_if(in_state(AppState::Chat)),
            );
    }
}

#[expect(clippy::needless_pass_by_value)]
fn on_username_set(username: On<PrimaryUserNameSet>, mut commands: Commands) {
    commands.insert_resource(IrcServer::new(username.0.clone()));
}

fn teardown(mut commands: Commands, users: Query<Entity, With<User>>) {
    for user in users {
        commands.entity(user).despawn();
    }
    commands.remove_resource::<IrcServer>();
}

struct WsSender(SplitSink<ws::WebSocketStream, ws::Message>);

impl WsSender {
    async fn send(&mut self, command: &Command) -> ws::error::Result<()> {
        self.0.send(ws::Message::text(String::from(command))).await
    }
}

#[derive(Debug)]
enum IrcEvent {
    PrimaryUser { nick: String, name: String },
    AddUser { nick: String },
    ChangeName { previous_nick: String, nick: String },
    UserJoined { nick: String },
    Part { nick: String },
    Quit { nick: String },
    Message { nick: String, message: String },
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
    pub fn new(user_name: String) -> Self {
        let (bevy_tx, bevy_rx) = async_channel::unbounded();
        let (irc_tx, irc_rx) = async_channel::unbounded();
        Self {
            tx: bevy_tx,
            rx: irc_rx,
            _task: IoTaskPool::get().spawn(async move {
                if let Err(e) = Self::serve(user_name, bevy_rx, irc_tx).await {
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
        user_name: String,
        bevy_rx: async_channel::Receiver<IrcControlMessage>,
        irc_tx: async_channel::Sender<IrcEvent>,
    ) -> Result<(), BevyError> {
        let stream = ws::connect_with_protocols(IRC_SERVER_URL, &["text.ircv3.net"]).await?;
        let (ws_tx, mut ws_rx) = stream.split();
        let mut ws_tx = WsSender(ws_tx);

        let mut server_nick = user_name.clone();
        server_nick.retain(|c| !c.is_alphanumeric());

        // Send a CAP END to signify that we're IRCv3-compliant (and to end negotiations!).
        ws_tx
            .send(&Command::CAP(None, CapSubCommand::END, None, None))
            .await?;

        ws_tx
            .send(&Command::USER(
                server_nick.clone(),
                "0".into(),
                server_nick.clone(),
            ))
            .await?;

        ws_tx.send(&Command::NICK(server_nick.clone())).await?;

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
                            .send(&Command::JOIN(IRC_CHANNEL.to_string(), None, None))
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
                name: user_name,
            })
            .await?;

        let events = stream::select(
            ws_rx.map(StreamMessage::WsMessage),
            bevy_rx.map(StreamMessage::IrcControl),
        );
        Self::event_loop(events, server_nick, ws_tx, irc_tx).await
    }

    #[expect(clippy::too_many_lines)]
    async fn event_loop<S>(
        events: S,
        mut server_nick: String,
        mut ws_tx: WsSender,
        irc_tx: async_channel::Sender<IrcEvent>,
    ) -> Result<()>
    where
        S: Stream<Item = StreamMessage>,
    {
        let mut nick_prefixes = None;

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
                                command: Command::Response(Response::RPL_ISUPPORT, ref args),
                                ..
                            } => {
                                if let Some(prefix) =
                                    args.iter().find(|&a| a.starts_with("PREFIX="))
                                    // Looks like "PREFIX=(qaohv)~&@%6+"
                                    && let Some((_, prefixes)) = prefix.split_once(')')
                                {
                                    nick_prefixes = Some(prefixes.to_string());
                                }
                            }
                            IrcMessage {
                                command: Command::Response(Response::RPL_NAMREPLY, ref args),
                                ..
                            } if args.len() == 4 => {
                                for mut nick in args[3].split(' ') {
                                    if let Some(ref prefixes) = nick_prefixes
                                        && let Some(prefix) = nick.chars().next()
                                        && prefixes.contains(prefix)
                                    {
                                        nick = nick.split_at(1).1;
                                    }
                                    if nick != server_nick {
                                        irc_tx
                                            .send(IrcEvent::AddUser { nick: nick.into() })
                                            .await?;
                                    }
                                }
                            }
                            IrcMessage {
                                command: Command::JOIN(..),
                                prefix: Some(Prefix::Nickname(nick, ..)),
                                ..
                            } => {
                                irc_tx.send(IrcEvent::UserJoined { nick }).await?;
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
                                if previous_nick == server_nick {
                                    server_nick = nick.clone();
                                }
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
                            ws_tx
                                .send(&Command::PART(IRC_CHANNEL.to_string(), None))
                                .await?;
                        }
                        IrcControlMessage::Message { message } => {
                            ws_tx
                                .send(&Command::PRIVMSG(IRC_CHANNEL.to_string(), message))
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
            IrcEvent::AddUser { nick } => {
                let entity = commands.spawn(User).id();
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
            IrcEvent::UserJoined { nick } => {
                if !server.users.contains_key(&nick) {
                    let entity = commands.spawn(User).id();
                    server.users.insert(nick, entity);
                }
                commands.trigger(UserJoined);
            }
            IrcEvent::Part { nick } | IrcEvent::Quit { nick } => {
                if let Some(&user_entity) = server.users.get(&nick) {
                    commands.entity(user_entity).despawn();
                    server.users.remove(&nick);
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
        }
    }
}
