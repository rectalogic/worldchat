use bevy::{ecs::relationship::Relationship, prelude::*};

use crate::{
    UserMessage,
    irc::{
        ChannelOfServer, ChannelUsers, IrcControl, PrimaryUser, Server, ServerChannels, UserAdded,
        UserNameChanged, UserOfChannel,
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

fn on_user_added(
    added: On<UserAdded>,
    mut commands: Commands,
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

    // Broadcast our position when any user joins
    if added.joined
        && let Ok(server) = servers.get(added.server_entity)
    {
        server.send(IrcControl::Message {
            channel: added.channel_name.to_string(),
            message: "POSITION".into(), // XXX encode our (server/main user) position
        })?;
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
