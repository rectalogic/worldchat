use bevy::prelude::*;
use iroh_gossip::TopicId;

#[derive(Component)]
pub struct ChatRoom {
    name: String,
    topic_id: TopicId,
    bootstrap_ids: Vec<pkarr::PublicKey>,
}

impl ChatRoom {
    pub fn new(name: String, topic_id: TopicId, bootstrap_ids: Vec<pkarr::PublicKey>) -> Self {
        Self {
            topic_id,
            name,
            bootstrap_ids,
        }
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    pub fn bootstrap_ids(&self) -> &[pkarr::PublicKey] {
        &self.bootstrap_ids
    }
}
