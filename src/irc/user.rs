use super::channel::ChannelUsers;
use bevy::prelude::*;

#[derive(Component, Debug)]
#[relationship(relationship_target = ChannelUsers)]
pub struct UserOfChannel(Entity);

#[derive(EntityEvent)]
pub struct UserMessage {
    #[event_target]
    pub user_entity: Entity,
    pub message: String,
}
