use crate::{irc::IrcServerPlugin, world::WorldPlugin};
use bevy::prelude::*;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultPlugins, IrcServerPlugin, WorldPlugin))
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
