use std::str::FromStr;

use bevy::prelude::*;
use iroh::EndpointId;
use iroh_gossip::TopicId;
use pkarr::{
    Client,
    dns::rdata::{CNAME, RData},
};

//XXX need Task for each room to get the Chat resource and publish it's endpoint_id to one of the wellknown CNAMEs
//XXX need a system to drive that task

pub fn plugin(app: &mut App) {
    // app.add_systems(Update, publish_endpoint_id);
}

#[derive(Component)]
pub struct ChatRoom {
    name: String,
    topic_id: TopicId,
    well_known_peer_cnames: Vec<pkarr::PublicKey>,
}

impl ChatRoom {
    pub fn new(
        name: String,
        topic_id: TopicId,
        well_known_peer_cnames: Vec<pkarr::PublicKey>,
    ) -> Self {
        Self {
            topic_id,
            name,
            well_known_peer_cnames,
        }
    }
}

async fn publish_endpoint_id() {}

async fn resolve_bootstrap_endpoint_ids(
    client: &Client,
    well_known_peer_cnames: &[pkarr::PublicKey],
) -> Vec<EndpointId> {
    let mut bootstrap_ids = Vec::new();
    for ep in well_known_peer_cnames {
        if let Some(packet) = client.resolve_most_recent(ep).await {
            bootstrap_ids.extend(packet.all_resource_records().filter_map(|record| {
                if let RData::CNAME(CNAME(name)) = &record.rdata
                    && let Some(bytes) = name.as_bytes().next()
                    && let Ok(endpoint_str) = str::from_utf8(bytes)
                {
                    EndpointId::from_str(endpoint_str).ok()
                } else {
                    None
                }
            }));
        }
    }
    bootstrap_ids
}
