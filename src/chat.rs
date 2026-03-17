use bevy::prelude::*;

mod join;
mod room;
mod user;

pub fn plugin(app: &mut App) {
    app.add_plugins((user::plugin, room::plugin, join::plugin));
}
