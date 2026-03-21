use crate::chat::{ChatPlugin, join::ChatRoomEvent, room::ChatRoom};
use bevy::prelude::*;
use iroh::SecretKey;

// Generate with "cargo run --bin keypair"
static TOPIC_KEYPAIR: [u8; 32] = [
    201, 245, 254, 101, 19, 40, 114, 181, 87, 252, 79, 245, 160, 127, 138, 116, 75, 103, 138, 129,
    237, 134, 176, 223, 152, 165, 216, 198, 253, 224, 219, 255,
];

pub struct AppPlugin;

impl Plugin for AppPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins,
            ChatPlugin {
                secret_key: SecretKey::generate(&mut rand::rng()),
            },
        ))
        .add_systems(Startup, setup)
        .run();
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn(ChatRoom::new(pkarr::Keypair::from_secret_key(
            &TOPIC_KEYPAIR,
        )))
        .observe(|chat_event: On<ChatRoomEvent>| {
            dbg!(&chat_event);
        });
}
