use crate::{
    irc::{ChannelOfServer, IrcPlugin, Server, send_channel_message},
    world::WorldPlugin,
};
use bevy::prelude::*;

pub struct AppPlugin {
    pub user_name: String,
}

#[derive(Resource)]
struct UserName(String);

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        let mut user_name = self.user_name.clone();
        user_name.retain(|c| !c.is_whitespace());
        app.add_plugins((DefaultPlugins, IrcPlugin, WorldPlugin))
            .insert_resource(UserName(user_name))
            .add_systems(Startup, setup);
    }
}

// We support multiple servers and channels per server - would need a way to expose this to the HTML and UI
static SERVER_URL: &str = "wss://fiery.swiftirc.net:4443";
static CHANNEL: &str = "#bevyworldchat";

pub fn send_message(message: String) -> Result<(), BevyError> {
    send_channel_message(SERVER_URL, CHANNEL.into(), message)
}

fn setup(mut commands: Commands, user_name: Res<UserName>) {
    commands.remove_resource::<UserName>();
    commands
        .spawn((
            Name::new(user_name.0.clone()),
            Server::new(SERVER_URL.into(), user_name.0.clone()),
        ))
        .with_related::<ChannelOfServer>(Name::new(CHANNEL));
}
