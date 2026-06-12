use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    app::AppState,
    irc::PrimaryUser,
    world::chat::{GridPosition, UserMoveQueue},
};

pub struct MovePlugin;

impl Plugin for MovePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, move_primary_user.run_if(in_state(AppState::Chat)));
    }
}

const DIRECTIONS: [(Dir2, IVec2); 8] = [
    (Dir2::NORTH, IVec2::new(0, 1)),
    (Dir2::NORTH_EAST, IVec2::new(1, 1)),
    (Dir2::EAST, IVec2::new(1, 0)),
    (Dir2::SOUTH_EAST, IVec2::new(1, -1)),
    (Dir2::SOUTH, IVec2::new(0, -1)),
    (Dir2::SOUTH_WEST, IVec2::new(-1, -1)),
    (Dir2::WEST, IVec2::new(-1, 0)),
    (Dir2::NORTH_WEST, IVec2::new(-1, 1)),
];

#[expect(clippy::needless_pass_by_value)]
fn move_primary_user(
    mut commands: Commands,
    user: Single<(Entity, &Transform, Option<&mut UserMoveQueue>), With<PrimaryUser>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    touches: Res<Touches>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    let mut position = None;

    if mouse_buttons.pressed(MouseButton::Left) {
        position = window.cursor_position();
    }
    if position.is_none() {
        position = touches.first_pressed_position();
    }

    if let Some(position) = position {
        let (camera, camera_transform) = *camera;
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, position) {
            let (user_entity, transform, mut move_queue) = user.into_inner();
            let delta = world_pos - transform.translation.truncate();
            let Ok(dir) = Dir2::new(delta) else {
                return;
            };
            if let Some((_, pos)) = DIRECTIONS
                .into_iter()
                .max_by(|(a, _), (b, _)| dir.dot(**a).partial_cmp(&dir.dot(**b)).unwrap())
            {
                let new_position = GridPosition(pos);
                if let Some(mut move_queue) = move_queue {
                    if Some(&new_position) == move_queue.back() {
                        return;
                    }
                    move_queue.push_back(new_position);
                } else {
                    commands
                        .entity(user_entity)
                        .insert(UserMoveQueue::new(new_position));
                }
            }
        }
    }
}
