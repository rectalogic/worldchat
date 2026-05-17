use crate::{irc::IrcServerPlugin, world::WorldPlugin};
use bevy::prelude::*;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "WorldChat".into(),
                    canvas: Some("#canvas".into()),
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            IrcServerPlugin,
            WorldPlugin,
        ))
        .init_state::<AppState>()
        .set_error_handler(bevy::ecs::error::error);
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Login,
    Chat,
    Error,
}
