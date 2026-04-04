use std::borrow::Cow;

use super::{
    message::IrcControl,
    server::{Server, ServerChannels},
    user::UserOfChannel,
};
use bevy::prelude::*;

pub struct ChannelPlugin;

impl Plugin for ChannelPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_remove).add_observer(on_add);
    }
}

#[derive(Component, Debug)]
#[relationship(relationship_target = ServerChannels)]
pub struct ChannelOfServer(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = UserOfChannel, linked_spawn)]
pub struct ChannelUsers(Vec<Entity>);

#[derive(Bundle, Debug)]
pub struct ChannelBundle {
    name: Name,
}

impl ChannelBundle {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        ChannelBundle {
            name: Name::new(name),
        }
    }
}

fn on_add(
    add: On<Add, ChannelOfServer>,
    query: Query<(&Name, &ChannelOfServer)>,
    mut servers: Query<&mut Server>,
) -> Result<(), BevyError> {
    if let Ok((name, channel_of_server)) = query.get(add.entity)
        && let Ok(mut server) = servers.get_mut(channel_of_server.0)
    {
        server.send(IrcControl::Join {
            channel: name.to_string(),
        })?;
    }
    Ok(())
}

fn on_remove(
    remove: On<Remove, ChannelOfServer>,
    query: Query<(&Name, &ChannelOfServer)>,
    mut servers: Query<&mut Server>,
) -> Result<(), BevyError> {
    if let Ok((name, channel_of_server)) = query.get(remove.entity)
        && let Ok(mut server) = servers.get_mut(channel_of_server.0)
    {
        server.send(IrcControl::Part {
            channel: name.to_string(),
        })?;
    }
    Ok(())
}
