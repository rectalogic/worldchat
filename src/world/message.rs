use bevy::{camera::primitives::Aabb, math::bounding::Aabb2d, prelude::*};

use crate::world::chat::GridPosition;

pub struct MessagePlugin;

impl Plugin for MessagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, sync_ui);
    }
}

// 1:1 relationship
#[derive(Component, Clone)]
#[relationship_target(relationship = SyncUiFollowerOf, linked_spawn)]
pub struct SyncUiLeader(Entity);

#[derive(Component, FromTemplate, Copy, Clone)]
#[relationship(relationship_target = SyncUiLeader)]
pub struct SyncUiFollowerOf(Entity);

pub fn configure_user_ui(
    name: impl Into<String>,
    position: GridPosition,
    mut commands: EntityCommands<'_>,
) {
    //XXX is this spawning too late, so Transform is inserted before SyncUiFollower?
    commands.apply_scene(message_ui(name, position));
}

fn message_ui(name: impl Into<String>, position: GridPosition) -> impl Scene {
    let background = Color::BLACK.with_alpha(0.5);
    let name = name.into();
    let user_name = name.clone();
    let position_vec = position.0;
    bsn! {
        Name(user_name)
        Text2d(name)
        // https://github.com/bevyengine/bevy/issues/24531
        GridPosition(position_vec)
        Transform::from(position)
        SyncUiLeader [
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::px(10.0, 10.0, 8.0, 6.0),
                width: px(300.0),
            }
            Visibility::Hidden
            BackgroundColor(background)
            TextColor(Color::WHITE)
            TextLayout {
                justify: Justify::Left,
                linebreak: LineBreak::WordBoundary
            }
        ]
    }
}

pub fn display_message(mut commands: EntityCommands<'_>, message: impl Into<String>) {
    //XXX need to mark the 2d target Transform as mutatded so we sync_ui
    // XXX ugh, sync_ui doesn't run because GlobalTransform changes before we add SyncUiLeader - so we never see the initial insert
    commands.insert((Text(message.into()), Visibility::Inherited));
}

fn sync_ui(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    target_query: Query<(&SyncUiLeader, &Aabb, &GlobalTransform), Changed<GlobalTransform>>,
    mut node_query: Query<&mut Node>,
) {
    let (camera, camera_transform) = camera_query.into_inner();
    for (target, aabb, world_transform) in &target_query {
        for node_entity in target.iter() {
            let Ok(mut node) = node_query.get_mut(node_entity) else {
                continue;
            };
            let world_pos = world_transform.translation();
            if let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, world_pos) {
                let aabb = Aabb2d::new(aabb.center.truncate(), aabb.half_extents.truncate());
                let offset = (aabb.max - aabb.min) / 2.0;
                node.left = Val::Px(viewport_pos.x - offset.x);
                node.top = Val::Px(viewport_pos.y + offset.y);
            }
        }
    }
}
