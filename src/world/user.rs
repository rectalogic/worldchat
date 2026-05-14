use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    User, UserMessage,
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
    users: Query<&Name, With<User>>,
) {
    if let Ok(name) = users.get(added.entity) {
        //XXX modify initial transform so user not always at 0,0
        commands
            .entity(added.entity)
            .insert(Text2d::new(name.as_str()));
    }
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

type UsersQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        Option<&'static Name>,
        Option<&'static PrimaryUser>,
    ),
    With<User>,
>;

#[expect(clippy::needless_pass_by_value)]
fn on_message(
    user_message: On<UserMessage>,
    mut commands: Commands,
    mut users: UsersQuery,
) -> Result<()> {
    let (info, message) = match user_message.message.split_once(' ') {
        None => (UserInfo::try_from(user_message.message.as_str())?, None),
        Some((info, message)) => (UserInfo::try_from(info)?, Some(message)),
    };

    // Don't modify primary user
    if let Ok((mut transform, name, primary)) = users.get_mut(user_message.user_entity)
        && primary.is_none()
    {
        //XXX should animate lerp to new position (and queue up position changes)
        transform.translation.x = info.position.x;
        transform.translation.y = info.position.y;
        if let Some(new_name) = info.name
            && name.is_none()
        {
            commands
                .entity(user_message.user_entity)
                .insert((Name::new(new_name.clone()), Text2d::new(new_name)));
        }
    }

    if let Some(message) = message {
        // XXX add visual message component displaying last message
    }

    Ok(())
}
