use bevy::prelude::*;

mod member;
mod room;
mod user;

pub fn plugin(app: &mut App) {
    app.add_plugins((user::plugin, room::plugin, member::plugin));
}
