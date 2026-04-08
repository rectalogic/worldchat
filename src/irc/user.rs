use super::channel::ChannelUsers;
use bevy::prelude::*;

#[derive(Component, Debug)]
#[relationship(relationship_target = ChannelUsers)]
pub struct UserOfChannel(Entity);

#[derive(EntityEvent, Debug)]
pub struct UserMessage {
    #[event_target]
    pub channel_entity: Entity,
    pub user_entity: Option<Entity>,
    pub server_entity: Entity,
    pub user_name: Name,
    pub message: String,
}
