use bevy::prelude::*;

use crate::{app::AppState, irc::PrimaryUserNameSet, world::form};

pub struct LoginPlugin;

impl Plugin for LoginPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, scene.spawn())
            .add_systems(OnExit(AppState::Login), teardown);
    }
}

fn scene() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [
            Node {
                width: percent(90),
            }
            Children [
                form::ui("Join")
                on(submit_join)
            ]
        ]
    }
}

#[expect(clippy::needless_pass_by_value)]
fn submit_join(
    event: On<form::SubmitTextEvent>,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
) {
    commands.trigger(PrimaryUserNameSet(event.value.clone()));
    next_state.set(AppState::Chat);
}

fn teardown(mut commands: Commands, nodes: Query<Entity, With<Node>>) {
    for node in nodes {
        commands.entity(node).despawn();
    }
}

//XXX trigger PrimaryUserNameSet and transition state when login entered
