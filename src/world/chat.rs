use std::collections::VecDeque;

use base64::{Engine as _, engine::general_purpose::STANDARD_NO_PAD};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    irc::{IrcControlMessage, IrcServer, PrimaryUser, User, UserJoined, UserMessage},
    world::form,
};

pub struct ChatPlugin;

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_primary_user_added)
            .add_observer(on_user_added)
            .add_observer(on_user_joined)
            .add_observer(on_message)
            .add_systems(OnEnter(AppState::Chat), scene.spawn())
            .add_systems(Update, update_moving_users.run_if(in_state(AppState::Chat)));
    }
}

const GRID_CELL: f32 = 32.0;

#[derive(Component, Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct GridPosition(pub IVec2);

impl GridPosition {
    fn as_translation(self) -> Vec3 {
        #[expect(clippy::cast_precision_loss)]
        Vec3::new(
            (self.0.x * GRID_CELL as i32) as f32,
            (self.0.y * GRID_CELL as i32) as f32,
            0.0,
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum UserMessageData {
    Broadcast {
        name: String,
        position: GridPosition,
    },
    Position(GridPosition),
    Message,
}

impl UserMessageData {
    fn base64(&self) -> Result<String> {
        Ok(STANDARD_NO_PAD.encode(postcard::to_allocvec(self)?))
    }
}

impl From<&Transform> for GridPosition {
    fn from(transform: &Transform) -> Self {
        GridPosition(IVec2::new(
            (transform.translation.x / GRID_CELL) as i32,
            (transform.translation.y / GRID_CELL) as i32,
        ))
    }
}

impl From<Vec2> for GridPosition {
    fn from(position: Vec2) -> Self {
        GridPosition(IVec2::new(
            (position.x / GRID_CELL) as i32,
            (position.y / GRID_CELL) as i32,
        ))
    }
}

impl From<GridPosition> for Transform {
    fn from(position: GridPosition) -> Self {
        Transform::from_translation(position.as_translation())
    }
}

impl TryFrom<&str> for UserMessageData {
    type Error = BevyError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let bytes = STANDARD_NO_PAD.decode(value)?;
        Ok(postcard::from_bytes::<UserMessageData>(&bytes)?)
    }
}

#[derive(Component, Default, Debug, Deref, DerefMut)]
pub struct UserMoveQueue(VecDeque<GridPosition>);

impl UserMoveQueue {
    pub fn new(position: GridPosition) -> Self {
        let mut q = Self::default();
        q.push_back(position);
        q
    }
}

fn scene() -> impl Scene {
    bsn! {
        #ChatMessage
        DespawnOnExit<AppState>(AppState::Chat)
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::FlexEnd,
            justify_content: JustifyContent::Center,
        }
        Children [
            Node {
                width: percent(90),
            }
            Children [
                form::ui("Message")
                on(submit_message)
            ]
        ]
    }
}

#[expect(clippy::needless_pass_by_value)]
fn submit_message(text_event: On<form::SubmitTextEvent>, server: Res<IrcServer>) -> Result<()> {
    server.send(IrcControlMessage::Message {
        message: format!(
            "{} {}",
            UserMessageData::Message.base64()?,
            text_event.value.clone()
        ),
    })?;
    Ok(())
}

#[expect(clippy::needless_pass_by_value)]
fn on_primary_user_added(
    added: On<Add, PrimaryUser>,
    mut commands: Commands,
    users: Query<&Name, With<User>>,
) {
    if let Ok(name) = users.get(added.entity) {
        //XXX modify initial transform/GridPosition so user not always at 0,0
        commands
            .entity(added.entity)
            .insert((Text2d::new(name.as_str()), GridPosition(IVec2::default())));
    }
}

#[expect(clippy::needless_pass_by_value)]
fn on_user_added(added: On<Add, User>, mut commands: Commands) {
    commands
        .entity(added.entity)
        .insert(DespawnOnExit(AppState::Chat));
}

#[expect(clippy::needless_pass_by_value)]
fn on_user_joined(
    _joined: On<UserJoined>,
    primary_user: Single<(&Name, &GridPosition), With<PrimaryUser>>,
    server: Res<IrcServer>,
) -> Result<()> {
    let (name, &grid_position) = *primary_user;
    // Broadcast our position and name in channel when any other user joins
    server.send(IrcControlMessage::Message {
        message: UserMessageData::Broadcast {
            name: name.to_string(),
            position: grid_position,
        }
        .base64()?,
    })?;
    Ok(())
}

type UsersQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static mut UserMoveQueue>,
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
    let (message_data, message) = match user_message.message.split_once(' ') {
        None => (
            UserMessageData::try_from(user_message.message.as_str())?,
            None,
        ),
        Some((message_data, message)) => (UserMessageData::try_from(message_data)?, Some(message)),
    };

    let Ok((move_queue, user_name, primary_user)) = users.get_mut(user_message.user_entity) else {
        return Ok(());
    };

    match message_data {
        UserMessageData::Broadcast { name, position } => {
            if user_name.is_none() {
                // Warp to position - new user
                commands.entity(user_message.user_entity).insert((
                    Name::new(name.clone()),
                    Text2d::new(name),
                    Transform::from(position),
                    position,
                ));
            }
        }
        UserMessageData::Position(position) => {
            if primary_user.is_none() {
                if let Some(mut move_queue) = move_queue {
                    move_queue.push_back(position);
                } else {
                    commands
                        .entity(user_message.user_entity)
                        .insert(UserMoveQueue::new(position));
                }
            }
        }
        UserMessageData::Message => {}
    }

    if let Some(message) = message {
        // XXX add visual message component displaying last message
    }
    Ok(())
}

#[expect(clippy::needless_pass_by_value)]
fn update_moving_users(
    mut commands: Commands,
    moving_users: Query<(Entity, &mut UserMoveQueue, &mut Transform, &GridPosition)>,
    time: Res<Time>,
) {
    // GridPosition is starting pos, queue front has target pos, transform is where we are
    // animate and when we reach target, pop queue, and remove if empty
    for (entity, mut move_queue, mut transform, start_position) in moving_users {
        let Some(target_position) = move_queue.front() else {
            commands.entity(entity).remove::<UserMoveQueue>();
            continue;
        };
        let target_translation = target_position.as_translation();
        let mut translation = start_position.as_translation();
        translation.smooth_nudge(&target_translation, 3.0, time.delta_secs());
        if (target_translation - translation).length() <= f32::EPSILON {
            transform.translation = target_translation;
            move_queue.pop_front();
        } else {
            transform.translation = translation;
        }
    }
}
