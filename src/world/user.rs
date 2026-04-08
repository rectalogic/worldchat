use bevy::{ecs::relationship::Relationship, prelude::*};

use crate::{
    UserMessage,
    irc::{ChannelOfServer, IrcControl, Server, UserJoined, UserOfChannel},
};

pub struct UserPlugin;

impl Plugin for UserPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_user_joined).add_observer(on_message);
    }
}

fn on_user_joined(
    joined: On<UserJoined>,
    mut commands: Commands,
    mut servers: Query<&mut Server>,
) -> Result<(), BevyError> {
    // Broadcast our position when any user joins
    if let Ok(mut server) = servers.get_mut(joined.server_entity) {
        server.send(IrcControl::Message {
            channel: joined.channel_name.to_string(),
            message: "POSITION".into(), // XXX encode our (server/main user) position
        })?;
        // Update our name
        if server.user() == joined.user_name {
            commands.entity(joined.server_entity).insert((
                Text2d::new(joined.user_name.as_str()),
                Name::new(joined.user_name.clone()),
            ));
        }
    }
    Ok(())
}

fn on_message(message: On<UserMessage>, mut commands: Commands) {
    if let Some(user_entity) = message.user_entity {
        // XXX message could be from us or another user
        // XXX decode position from message and set Transform
    } else {
        // Message was from an unrecognized user, spawn new user in channel
        commands
            .entity(message.channel_entity)
            .with_related::<UserOfChannel>((
                Text2d::new(message.user_name.as_str()),
                message.user_name.clone(),
            ));
        //XXX decode position from message and set Transform
    }

    // XXX add visual message component displaying last message
}
