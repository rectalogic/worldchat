use super::server::Server;
use bevy::prelude::*;

pub struct ChannelPlugin;

impl Plugin for ChannelPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_add)
            .add_observer(on_remove)
            .add_observer(on_add)
            .add_systems(Update, handle_server_events);
    }
}

#[derive(Component, Debug)]
pub struct Channel {
    name: String,
}

//XXX listen for server events, fire EntityEvents for the corresponding channel
fn handle_server_events() {}

fn on_add(add: On<Add, Channel>, query: Query<&Channel>, mut server: ResMut<Server>) {
    if let Ok(channel) = query.get(add.entity) {
        server.join((add.entity, channel.name.clone()));
    }
}

fn on_remove(remove: On<Remove, Channel>, query: Query<&Channel>, mut server: ResMut<Server>) {
    if let Ok(channel) = query.get(remove.entity) {
        server.leave((remove.entity, channel.name.clone()));
    }
}
