use bevy::prelude::*;
use iroh::SecretKey;
use worldchat::{ChatPlugin, ChatRoom, ChatRoomEvent};

// Generate with "cargo run --bin topicid"
static TOPIC_ID: [u8; 32] = [
    243, 238, 25, 48, 10, 203, 166, 58, 142, 135, 163, 148, 206, 79, 107, 37, 23, 167, 147, 173,
    48, 62, 46, 234, 167, 34, 155, 30, 61, 76, 234, 33,
];
// Generate with "cargo run --bin pubkey"
static BOOTSTRAP_IDS: [[u8; 32]; 3] = [
    [
        124, 109, 81, 227, 232, 13, 186, 189, 241, 60, 114, 204, 66, 247, 168, 153, 168, 242, 219,
        90, 170, 2, 153, 127, 203, 150, 54, 33, 225, 184, 112, 52,
    ],
    [
        96, 199, 2, 36, 41, 221, 145, 255, 57, 178, 174, 134, 133, 250, 8, 120, 115, 121, 2, 199,
        31, 179, 51, 241, 127, 182, 125, 17, 164, 39, 213, 122,
    ],
    [
        10, 157, 51, 189, 221, 148, 227, 249, 197, 47, 205, 82, 78, 183, 64, 56, 38, 178, 108, 165,
        145, 82, 15, 235, 37, 91, 132, 185, 136, 166, 204, 104,
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
    let bootstrap_ids: Vec<pkarr::PublicKey> = BOOTSTRAP_IDS
        .iter()
        .map(pkarr::PublicKey::try_from)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    commands
        .spawn(ChatRoom::new("foo".into(), TOPIC_ID.into(), bootstrap_ids))
        .observe(|chat_event: On<ChatRoomEvent>| {
            dbg!(&chat_event);
        });
}
