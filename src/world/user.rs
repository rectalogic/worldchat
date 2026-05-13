use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    UserMessage,
    irc::{IrcControlMessage, IrcServer, PrimaryUser, UserJoined},
};

pub struct UserPlugin;

impl Plugin for UserPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_primary_user_added)
            .add_observer(on_user_joined)
            .add_observer(on_message);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserInfo {
    pub name: Option<String>,
    pub position: UserPosition,
}

impl UserInfo {
    pub fn base64(&self) -> Result<String> {
        Ok(STANDARD_NO_PAD.encode(postcard::to_allocvec(self)?))
    }
}

impl From<&Transform> for UserPosition {
    fn from(transform: &Transform) -> Self {
        UserPosition {
            x: transform.translation.x,
            y: transform.translation.y,
        }
    }
}

impl TryFrom<&str> for UserInfo {
    type Error = BevyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = STANDARD_NO_PAD.decode(value)?;
        Ok(postcard::from_bytes::<UserInfo>(&bytes)?)
    }
}

#[expect(clippy::needless_pass_by_value)]
fn on_primary_user_added(
    added: On<Add, PrimaryUser>,
    mut commands: Commands,
    primary_user: Single<(Entity, &Name), With<PrimaryUser>>,
) {
    let (entity, name) = *primary_user;
    //XXX modify initial transform so user not always at 0,0
    commands.entity(entity).insert(Text2d::new(name.as_str()));
}

#[expect(clippy::needless_pass_by_value)]
fn on_user_joined(
    _joined: On<UserJoined>,
    primary_user: Single<(&Name, &Transform), With<PrimaryUser>>,
    server: Res<IrcServer>,
) -> Result<()> {
    let (name, transform) = *primary_user;

    // Broadcast our position and name in channel when any other user joins
    server.send(IrcControlMessage::Message {
        message: UserInfo {
            name: Some(name.to_string()),
            position: UserPosition::from(transform),
        }
        .base64()?,
    })?;
    Ok(())
}

fn on_message(message: On<UserMessage>, mut commands: Commands) {
    // XXX decode position from message and set Transform on message.user_entity

    // XXX add visual message component displaying last message
}
