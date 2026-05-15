use bevy::prelude::*;

mod chat;
mod form;
mod login;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((form::FormPlugin, login::LoginPlugin, chat::ChatPlugin))
            .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
