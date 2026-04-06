use bevy::{ecs::relationship::Relationship, prelude::*};

use crate::irc::{ChannelOfServer, IrcControl, Server, UserOfChannel};

pub struct UserPlugin;

impl Plugin for UserPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_add);
    }
}

fn on_add(
    add: On<Add, UserOfChannel>,
    mut commands: Commands,
    users: Query<(&Name, &UserOfChannel)>,
    channels: Query<(&Name, &ChannelOfServer)>,
    mut servers: Query<(&Name, &mut Server)>,
) -> Result<(), BevyError> {
    // When any remote user joins, we broadcast our current position
    if let Ok((user_name, user_of_channel)) = users.get(add.entity)
        && let Ok((channel_name, channel_of_server)) = channels.get(user_of_channel.get())
        && let Ok((server_user, mut server)) = servers.get_mut(channel_of_server.get())
    {
        if server_user == user_name {
            // XXX visually distinguish our user from remote users
            commands.entity(add.entity).insert(Text2d::new(server_user));
        } else {
            commands.entity(add.entity).insert(Text2d::new(user_name));
        }
        server.send(IrcControl::Message {
            channel: channel_name.to_string(),
            message: "POSITION".into(), // XXX encode our (server/main user) position
        })?;
    }
    Ok(())
}
