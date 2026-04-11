use std::{cell::RefCell, sync::LazyLock};

use crate::{
    irc::{
        ChannelOfServer, ChannelUsers, IrcControl, IrcPlugin, PrimaryUser, Server, ServerChannels,
        UserOfChannel, find_relationship_source_named,
    },
    world::{WorldPlugin, WorldPosition},
};
use bevy::prelude::*;

pub struct AppPlugin {
    pub user_name: String,
}

#[derive(Resource)]
struct UserName(String);

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        let mut user_name = self.user_name.clone();
        user_name.retain(|c| !c.is_whitespace());
        app.add_plugins((DefaultPlugins, IrcPlugin, WorldPlugin))
            .insert_resource(UserName(user_name))
            .add_systems(Startup, setup)
            .add_systems(Update, poll_external_messages);
    }
}

#[derive(Default)]
struct ExternalMessage {
    server_name: Name,
    channel_name: Name,
    message: String,
}

thread_local! {
    static EXTERNAL_MESSAGE_CHANNELS: RefCell<(async_channel::Sender<ExternalMessage>, async_channel::Receiver<ExternalMessage>)> = RefCell::new(async_channel::unbounded());
}

// We support multiple servers and channels per server - would need a way to expose this to the HTML and UI
static SERVER_URL: LazyLock<Name> = LazyLock::new(|| Name::new("wss://fiery.swiftirc.net:4443"));
static CHANNEL: LazyLock<Name> = LazyLock::new(|| Name::new("#bevyworldchat"));

pub fn send_message(message: String) -> Result<(), BevyError> {
    EXTERNAL_MESSAGE_CHANNELS.with_borrow(|(tx, _)| {
        tx.try_send(ExternalMessage {
            server_name: (*SERVER_URL).clone(),
            channel_name: (*CHANNEL).clone(),
            message,
        })
    })?;
    Ok(())
}

fn setup(mut commands: Commands, user_name: Res<UserName>) {
    commands.remove_resource::<UserName>();
    commands
        .spawn((
            (*SERVER_URL).clone(),
            Server::new((*SERVER_URL).to_string(), user_name.0.clone()),
        ))
        .with_related::<ChannelOfServer>((*CHANNEL).clone());
}

fn poll_external_messages(
    servers: Query<(Entity, &Name, &Server)>,
    server_channels: Query<&ServerChannels>,
    channels: Query<(Entity, &Name), With<ChannelOfServer>>,
    channel_users: Query<&ChannelUsers>,
    users: Query<&Transform, (With<UserOfChannel>, With<PrimaryUser>)>,
) -> Result<(), BevyError> {
    EXTERNAL_MESSAGE_CHANNELS.with_borrow(|(_, rx)| -> Result<(), BevyError> {
        if let Ok(message) = rx.try_recv()
            && let Some((server_entity, server)) =
                servers.iter().find_map(|(entity, name, server)| {
                    if *name == message.server_name {
                        Some((entity, server))
                    } else {
                        None
                    }
                })
            && let Some(channel_entity) = find_relationship_source_named(
                &message.channel_name,
                server_entity,
                server_channels,
                channels,
            )
            && let Some(user_transform) = channel_users
                .relationship_sources::<ChannelUsers>(channel_entity)
                .find_map(|user_entity| users.get(user_entity).ok())
        {
            server.send(IrcControl::Message {
                channel: message.channel_name.to_string(),
                message: format!(
                    "{} {}",
                    WorldPosition::from(user_transform).base64()?,
                    message.message
                ),
            })?;
        }
        Ok(())
    })?;
    Ok(())
}
