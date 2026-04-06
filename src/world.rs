use bevy::prelude::*;

mod user;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(user::UserPlugin);
    }
}
