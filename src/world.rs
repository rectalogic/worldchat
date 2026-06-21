use bevy::prelude::*;

mod chat;
mod error;
mod form;
mod login;
mod message;
mod movement;
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            form::FormPlugin,
            login::LoginPlugin,
            chat::ChatPlugin,
            movement::MovePlugin,
            error::ErrorPlugin,
            message::MessagePlugin,
        ))
        .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
