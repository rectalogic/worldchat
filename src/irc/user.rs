use std::borrow::Cow;

use super::channel::ChannelUsers;
use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct User;

#[derive(Component, Debug)]
#[relationship(relationship_target = ChannelUsers)]
pub struct UserOfChannel(Entity);

#[derive(Bundle, Debug)]
pub struct UserBundle {
    name: Name,
    user: User,
}

impl UserBundle {
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        UserBundle {
            name: Name::new(name),
            user: User,
        }
    }
}
