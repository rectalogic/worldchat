use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    UserMessage,
    irc::{
        ChannelUsers, IrcControl, PrimaryUser, Server, ServerChannels, UserAdded, UserNameChanged,
        UserOfChannel,
    },
};

pub struct UserPlugin;

impl Plugin for UserPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_user_added)
            .add_observer(on_name_changed)
            .add_observer(on_message);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorldPosition {
    pub x: f32,
    pub y: f32,
}

impl WorldPosition {
    pub fn base64(&self) -> Result<String, BevyError> {
        Ok(STANDARD_NO_PAD.encode(postcard::to_allocvec(self)?))
    }
}

impl From<&Transform> for WorldPosition {
    fn from(transform: &Transform) -> Self {
        WorldPosition {
            x: transform.translation.x,
            y: transform.translation.y,
        }
    }
}

impl TryFrom<&str> for WorldPosition {
    type Error = BevyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = STANDARD_NO_PAD.decode(value)?;
        Ok(postcard::from_bytes::<WorldPosition>(&bytes)?)
    }
}

fn on_user_added(
    added: On<UserAdded>,
    mut commands: Commands,
    primary_users: Query<(Entity, &Transform), With<PrimaryUser>>,
    users_of_channel: Query<&UserOfChannel>,
    servers: Query<&Server>,
) -> Result<(), BevyError> {
    let mut entity_commands = commands.entity(added.channel_entity);
    entity_commands.with_related::<UserOfChannel>((
        Text2d::new(added.user_name.as_str()),
        added.user_name.clone(),
    ));
    if added.primary {
        entity_commands.insert(PrimaryUser);
    }

    // Broadcast primary users position in channel when any user joins
    if added.joined
        && let Ok(server) = servers.get(added.server_entity)
    {
        for (primary_user_entity, transform) in primary_users {
            if let Some(channel_entity) = users_of_channel.related(primary_user_entity)
                && channel_entity == added.channel_entity
            {
                server.send(IrcControl::Message {
                    channel: added.channel_name.to_string(),
                    message: WorldPosition::from(transform).base64()?,
                })?;
            }
        }
    }
    Ok(())
}

fn on_name_changed(
    namechange: On<UserNameChanged>,
    mut commands: Commands,
    server_channels: Query<&ServerChannels>,
    channel_users: Query<&ChannelUsers>,
    users: Query<&Name>,
) {
    if let Ok(server_channels) = server_channels.get(namechange.server_entity) {
        for channel_entity in server_channels.iter() {
            if let Ok(channel_users) = channel_users.get(channel_entity) {
                for user_entity in channel_users.iter() {
                    if let Ok(user_name) = users.get(user_entity)
                        && *user_name == namechange.previous_name
                    {
                        commands.entity(user_entity).insert((
                            Text2d::new(namechange.name.as_str()),
                            namechange.name.clone(),
                        ));
                    }
                }
            }
        }
    }
}

fn on_message(message: On<UserMessage>, mut commands: Commands) {
    // XXX decode position from message and set Transform on message.user_entity

    // XXX add visual message component displaying last message
}
