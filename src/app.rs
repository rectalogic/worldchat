use crate::irc::IrcPlugin;
use crate::irc::{Channel, ChannelOfServer, Server};
use bevy::prelude::*;

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DefaultPlugins, IrcPlugin))
            .add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn(Server::new(
            "wss://fiery.swiftirc.net:4443".into(),
            "user008".into(),
        ))
        .with_related::<ChannelOfServer>(Channel {
            name: "#bevyworldchat".into(),
        });
}
