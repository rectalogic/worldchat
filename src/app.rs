use std::{cell::RefCell, sync::LazyLock};

use crate::{
    irc::{
        ActiveChannel, ChannelOfServer, IrcControl, IrcPlugin, PrimaryUser, Server, UserOfChannel,
    },
    world::{WorldPlugin, WorldPosition},
};
use bevy::{ecs::relationship::Relationship, prelude::*};

pub struct AppPlugin {
    pub user_name: String,
}

#[derive(Resource)]
struct UserName(String);

#[derive(Resource, Deref)]
struct ExternalMessageReceiver(async_channel::Receiver<ExternalMessage>);

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        let mut user_name = self.user_name.clone();
        user_name.retain(|c| !c.is_whitespace());
        app.add_plugins((DefaultPlugins, IrcPlugin, WorldPlugin))
            .insert_resource(UserName(user_name))
            .insert_resource(ExternalMessageReceiver(
                EXTERNAL_MESSAGE_CHANNELS.with_borrow(|(_, rx)| rx.clone()),
            ))
            .add_systems(Startup, setup)
            .add_systems(Update, poll_external_messages);
    }
}

#[derive(Default)]
struct ExternalMessage {
    message: String,
}

thread_local! {
    static EXTERNAL_MESSAGE_CHANNELS: RefCell<(async_channel::Sender<ExternalMessage>, async_channel::Receiver<ExternalMessage>)> = RefCell::new(async_channel::unbounded());
}

// We support multiple servers and channels per server - would need a way to expose this to the HTML and UI
static SERVER_URL: LazyLock<Name> = LazyLock::new(|| Name::new("wss://fiery.swiftirc.net:4443"));
static CHANNEL: LazyLock<Name> = LazyLock::new(|| Name::new("#bevyworldchat"));

pub fn send_message(message: String) -> Result<(), BevyError> {
    EXTERNAL_MESSAGE_CHANNELS.with_borrow(|(tx, _)| tx.try_send(ExternalMessage { message }))?;
    Ok(())
}

fn setup(mut commands: Commands, user_name: Res<UserName>) {
    commands.remove_resource::<UserName>();
    commands
        .spawn((
            (*SERVER_URL).clone(),
            Server::new((*SERVER_URL).to_string(), user_name.0.clone()),
        ))
        // XXX expose a way for user to set ActiveChannel if more than one
        .with_related::<ChannelOfServer>(((*CHANNEL).clone(), ActiveChannel));
}

fn poll_external_messages(
    servers: Query<&Server>,
    active_channel: Single<(Entity, &ChannelOfServer, &Name), With<ActiveChannel>>,
    primary_users: Query<(&UserOfChannel, &Transform), With<PrimaryUser>>,
    receiver: Res<ExternalMessageReceiver>,
) -> Result<(), BevyError> {
    if let Ok(message) = receiver.try_recv()
        && let (active_channel_entity, channel_of_server, active_channel_name) = *active_channel
        && let Ok(server) = servers.get(channel_of_server.get())
        && let Some(user_transform) =
            primary_users
                .iter()
                .find_map(|(user_of_channel, transform)| {
                    if user_of_channel.get() == active_channel_entity {
                        Some(transform)
                    } else {
                        None
                    }
                })
    {
        server.send(IrcControl::Message {
            channel: active_channel_name.to_string(),
            message: format!(
                "{} {}",
                WorldPosition::from(user_transform).base64()?,
                message.message
            ),
        })?;
    }

    Ok(())
}
