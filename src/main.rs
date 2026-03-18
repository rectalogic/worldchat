use bevy::prelude::*;
use iroh::SecretKey;
use worldchat::{ChatPlugin, ChatRoom, ChatRoomEvent};

// Generate with "cargo run --bin topicid"
static TOPIC_ID: [u8; 32] = [
    243, 238, 25, 48, 10, 203, 166, 58, 142, 135, 163, 148, 206, 79, 107, 37, 23, 167, 147, 173,
    48, 62, 46, 234, 167, 34, 155, 30, 61, 76, 234, 33,
];
// Generate with "cargo run --bin keypair"
static BOOTSTRAP_KEYPAIRS: [[u8; 32]; 3] = [
    [
        201, 245, 254, 101, 19, 40, 114, 181, 87, 252, 79, 245, 160, 127, 138, 116, 75, 103, 138,
        129, 237, 134, 176, 223, 152, 165, 216, 198, 253, 224, 219, 255,
    ],
    [
        38, 254, 71, 112, 247, 49, 238, 215, 108, 23, 12, 100, 12, 237, 228, 69, 244, 42, 249, 73,
        144, 219, 33, 208, 42, 10, 114, 119, 171, 25, 49, 76,
    ],
    [
        131, 33, 90, 162, 176, 151, 250, 51, 48, 176, 248, 199, 206, 119, 29, 213, 53, 93, 178,
        170, 27, 201, 103, 37, 129, 19, 174, 188, 140, 177, 53, 77,
    ],
];

#[tokio::main]
async fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ChatPlugin {
                secret_key: SecretKey::generate(&mut rand::rng()),
            },
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let bootstrap_ids: Vec<pkarr::Keypair> = BOOTSTRAP_KEYPAIRS
        .iter()
        .map(pkarr::Keypair::from_secret_key)
        .collect();
    commands
        .spawn(ChatRoom::new("foo".into(), TOPIC_ID.into(), bootstrap_ids))
        .observe(|chat_event: On<ChatRoomEvent>| {
            dbg!(&chat_event);
        });
}
