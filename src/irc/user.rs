use std::borrow::Cow;

use super::channel::ChannelUsers;
use bevy::prelude::*;

#[derive(Component, Debug)]
#[relationship(relationship_target = ChannelUsers)]
pub struct UserOfChannel(Entity);

#[derive(Bundle, Debug)]
pub struct UserBundle {
    name: Name,
    channel: UserOfChannel,
}

impl UserBundle {
    pub fn new(name: impl Into<Cow<'static, str>>, channel: Entity) -> Self {
        UserBundle {
            name: Name::new(name),
            channel: UserOfChannel(channel),
        }
    }
}
