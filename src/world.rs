use bevy::prelude::*;

mod form;
mod login;
mod user;

pub use user::{UserInfo, UserPosition};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((form::FormPlugin, login::LoginPlugin, user::UserPlugin))
            .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
