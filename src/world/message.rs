use bevy::{camera::primitives::Aabb, math::bounding::Aabb2d, prelude::*};

pub struct MessagePlugin;

impl Plugin for MessagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, sync_ui);
    }
}

// 1:1 relationship
#[derive(Component, Clone)]
#[relationship_target(relationship = SyncUiFollower, linked_spawn)]
pub struct SyncUiLeader(Entity);

#[derive(Component, FromTemplate, Copy, Clone)]
#[relationship(relationship_target = SyncUiLeader)]
pub struct SyncUiFollower(Entity);

pub fn configure_user_ui(
    name: impl Into<String>,
    bundle: impl Bundle,
    mut commands: EntityCommands<'_>,
) {
    let name = name.into();
    commands
        .queue_spawn_related_scenes::<SyncUiLeader>(message_ui())
        .insert((bundle, Name::new(name.clone()), Text2d::new(name)));
}

fn message_ui() -> impl SceneList {
    let background = Color::BLACK.with_alpha(0.5);
    bsn_list! [
        (
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::px(10.0, 10.0, 8.0, 6.0),
                width: px(300.0),
            }
            BackgroundColor(background)
            Text
            TextColor(Color::WHITE)
            TextLayout {
                justify: Justify::Left,
                linebreak: LineBreak::WordBoundary
            }
        )
    ]
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
