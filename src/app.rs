use crate::irc::IrcPlugin;
use crate::irc::{ChannelBundle, ChannelOfServer, Server};
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
            "user008".into(), //XXX needs to come from user input
        ))
        .with_related::<ChannelOfServer>(ChannelBundle::new("#bevyworldchat"));
}
