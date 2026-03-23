use bevy::prelude::*;
use iroh::SecretKey;

pub mod join;
pub mod room;
mod user;

pub struct ChatPlugin {
    pub secret_key: SecretKey,
}

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            user::UserPlugin {
                secret_key: self.secret_key.clone(),
            },
            join::plugin,
        ));
    }
}

pub fn to_z32(bytes: &[u8]) -> String {
    base32::encode(base32::Alphabet::Z, bytes)
}
