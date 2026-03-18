use bevy::prelude::*;
use iroh_gossip::TopicId;

#[derive(Component)]
pub struct ChatRoom {
    name: String,
    topic_id: TopicId,
    bootstrap_keypairs: Vec<pkarr::Keypair>,
}

impl ChatRoom {
    pub fn new(name: String, topic_id: TopicId, bootstrap_keypairs: Vec<pkarr::Keypair>) -> Self {
        Self {
            topic_id,
            name,
            bootstrap_keypairs,
        }
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    pub fn bootstrap_keypairs(&self) -> &[pkarr::Keypair] {
        &self.bootstrap_keypairs
    }
}
