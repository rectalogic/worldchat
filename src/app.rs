use crate::{
    irc::{ChannelOfServer, IrcPlugin, Server},
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

fn setup(mut commands: Commands, user_name: Res<UserName>) {
    commands.remove_resource::<UserName>();
    commands
        .spawn((
            Name::new(user_name.0.clone()),
            Server::new("wss://fiery.swiftirc.net:4443".into(), user_name.0.clone()),
        ))
        .with_related::<ChannelOfServer>(Name::new("#bevyworldchat"));
}
