use std::cell::RefCell;

use super::{
    message::IrcControl,
    server::{Server, ServerChannels},
    user::UserOfChannel,
};
use bevy::{platform::collections::HashMap, prelude::*};

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

#[derive(EntityEvent)]
pub struct UserAdded {
    pub server_entity: Entity,
    #[event_target]
    pub channel_entity: Entity,
    pub channel_name: Name,
    pub user_name: Name,
    pub joined: bool,
    pub primary: bool,
}

thread_local! {
    // Map server urls to maps of channel names to server senders
    static SERVER_CHANNEL_MAP: RefCell<HashMap<String, HashMap<String, async_channel::Sender<IrcControl>>>> = const { RefCell::new(HashMap::new())};
}

pub fn send_channel_message(
    server_url: &str,
    channel: String,
    message: String,
) -> Result<(), BevyError> {
    SERVER_CHANNEL_MAP.with_borrow(|server_channel_map| {
        if let Some(channel_map) = server_channel_map.get(server_url)
            && let Some(tx) = channel_map.get(&channel)
        {
            tx.try_send(IrcControl::Message { channel, message })
        } else {
            Ok(())
        }
    })?;
    Ok(())
}

fn on_add(
    add: On<Add, ChannelOfServer>,
    query: Query<(&Name, &ChannelOfServer)>,
    servers: Query<&Server>,
) -> Result<(), BevyError> {
    if let Ok((name, channel_of_server)) = query.get(add.entity)
        && let Ok(server) = servers.get(channel_of_server.0)
    {
        server.send(IrcControl::Join {
            channel: name.to_string(),
        })?;
        SERVER_CHANNEL_MAP.with_borrow_mut(|server_channel_map| {
            if let Some(tx) = server.irc_tx() {
                let channel_map = server_channel_map
                    .entry_ref(server.server_url())
                    .or_insert(HashMap::new());
                channel_map.insert(name.to_string(), tx.clone());
            }
        });
    }
    Ok(())
}

fn on_remove(
    remove: On<Remove, ChannelOfServer>,
    query: Query<(&Name, &ChannelOfServer)>,
    servers: Query<&Server>,
) -> Result<(), BevyError> {
    if let Ok((name, channel_of_server)) = query.get(remove.entity)
        && let Ok(server) = servers.get(channel_of_server.0)
    {
        SERVER_CHANNEL_MAP.with_borrow_mut(|server_channel_map| {
            if let Some(channel_map) = server_channel_map.get_mut(server.server_url()) {
                channel_map.remove(name.as_str());
            }
        });
        server.send(IrcControl::Part {
            channel: name.to_string(),
        })?;
    }
    Ok(())
}
