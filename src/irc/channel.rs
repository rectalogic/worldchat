use super::{
    server::{Server, ServerChannels},
    user::UserOfChannel,
};
use bevy::prelude::*;

pub struct ChannelPlugin;

impl Plugin for ChannelPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_remove)
            .add_observer(on_add)
            .add_systems(Update, handle_server_events);
    }
}

#[derive(Component, Debug)]
#[relationship(relationship_target = ServerChannels)]
pub struct ChannelOfServer(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship = UserOfChannel, linked_spawn)]
pub struct ChannelUsers(Vec<Entity>);

#[derive(Component, Debug)]
pub struct Channel {
    pub name: String,
}

//XXX listen for server events, fire EntityEvents for the corresponding channel
fn handle_server_events() {}

//XXX move these into server, it can also maintain name->Entity map
// XXX similary for user->Entity, keep that in Channel
fn on_add(
    add: On<Add, Channel>,
    query: Query<(&Channel, &ChannelOfServer)>,
    mut servers: Query<&mut Server>,
) {
    if let Ok((channel, channel_of_server)) = query.get(add.entity)
        && let Ok(mut server) = servers.get_mut(channel_of_server.0)
    {
        server.join(channel.name.clone(), add.entity);
    }
}

fn on_remove(
    remove: On<Remove, Channel>,
    query: Query<(&Channel, &ChannelOfServer)>,
    mut servers: Query<&mut Server>,
) {
    if let Ok((channel, channel_of_server)) = query.get(remove.entity)
        && let Ok(mut server) = servers.get_mut(channel_of_server.0)
    {
        server.leave(channel.name.clone());
    }
}
