use crate::{
    irc::{ChannelOfServer, IrcPlugin, Server},
    world::WorldPlugin,
};
use bevy::prelude::*;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultPlugins, IrcPlugin, WorldPlugin))
            .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    let user_name = "user008"; //XXX needs to come from user input
    commands
        .spawn((
            Name::new(user_name),
            Server::new("wss://fiery.swiftirc.net:4443".into(), user_name.into()),
        ))
        .with_related::<ChannelOfServer>(Name::new("#bevyworldchat"));
}
