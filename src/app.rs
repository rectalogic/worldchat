use std::cell::RefCell;

use crate::{
    irc::{IrcControlMessage, IrcServer, IrcServerPlugin, PrimaryUser},
    world::{UserInfo, UserPosition, WorldPlugin},
};
use bevy::prelude::*;

pub struct AppPlugin {
    pub user_name: String,
}

#[derive(Resource, Deref)]
struct ExternalMessageReceiver(async_channel::Receiver<ExternalMessage>);

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        let mut user_name = self.user_name.clone();
        user_name.retain(|c| !c.is_whitespace());
        app.add_plugins((
            DefaultPlugins,
            IrcServerPlugin {
                server_url: "wss://fiery.swiftirc.net:4443".to_string(),
                channel: "#bevyworldchat".to_string(),
                user: user_name,
            },
            WorldPlugin,
        ))
        .set_error_handler(bevy::ecs::error::error)
        .insert_resource(ExternalMessageReceiver(
            EXTERNAL_MESSAGE_CHANNELS.with_borrow(|(_, rx)| rx.clone()),
        ))
        .add_systems(Update, poll_external_messages);
    }
}

#[derive(Default)]
struct ExternalMessage {
    message: String,
}

thread_local! {
    static EXTERNAL_MESSAGE_CHANNELS: RefCell<(async_channel::Sender<ExternalMessage>, async_channel::Receiver<ExternalMessage>)> = RefCell::new(async_channel::unbounded());
}

pub fn send_message(message: String) -> Result<(), BevyError> {
    EXTERNAL_MESSAGE_CHANNELS.with_borrow(|(tx, _)| tx.try_send(ExternalMessage { message }))?;
    Ok(())
}

#[expect(clippy::needless_pass_by_value)]
fn poll_external_messages(
    server: Res<IrcServer>,
    primary_user: Single<(&Name, &Transform), With<PrimaryUser>>,
    receiver: Res<ExternalMessageReceiver>,
) -> Result<(), BevyError> {
    if let Ok(message) = receiver.try_recv() {
        let (name, transform) = *primary_user;
        server.send(IrcControlMessage::Message {
            message: format!(
                "{} {}",
                UserInfo {
                    name: Some(name.to_string()),
                    position: UserPosition::from(transform),
                }
                .base64()?,
                message.message
            ),
        })?;
    }
    Ok(())
}
