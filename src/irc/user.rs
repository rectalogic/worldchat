use super::channel::ChannelUsers;
use bevy::prelude::*;

#[derive(Component, Debug)]
#[relationship(relationship_target = ChannelUsers)]
pub struct UserOfChannel(Entity);
