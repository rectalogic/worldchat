use bevy::prelude::*;
use iroh::SecretKey;
use worldchat::{ChatPlugin, ChatRoom, ChatRoomEvent};

// Generate with "cargo run --bin keypair"
static TOPIC_KEYPAIR: [u8; 32] = [
    201, 245, 254, 101, 19, 40, 114, 181, 87, 252, 79, 245, 160, 127, 138, 116, 75, 103, 138, 129,
    237, 134, 176, 223, 152, 165, 216, 198, 253, 224, 219, 255,
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
    commands
        .spawn(ChatRoom::new(
            "foo".into(),
            pkarr::Keypair::from_secret_key(&TOPIC_KEYPAIR),
        ))
        .observe(|chat_event: On<ChatRoomEvent>| {
            dbg!(&chat_event);
        });
}
