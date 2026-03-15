use std::str::FromStr;

use bevy::{prelude::*, tasks::IoTaskPool};
use iroh::{EndpointId, SecretKey};
use iroh_gossip::TopicId;
use pkarr::{
    Client,
    dns::rdata::{CNAME, RData},
};

pub struct ChatPlugin {
    pub secret_key: SecretKey,
    pub topic: String,
    pub well_known_peer_cnames: Vec<pkarr::PublicKey>,
}

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        let topic_id = TopicId::from_str(&self.topic).unwrap();

        let client = Client::builder().build().unwrap();
        IoTaskPool::get()
            .spawn(async move {
                let mut bootstrap_endpoint_ids = Vec::new();
                for ep in self.well_known_peer_cnames {
                    if let Some(packet) = client.resolve_most_recent(&ep).await {
                        bootstrap_endpoint_ids.extend(packet.all_resource_records().filter_map(
                            |record| {
                                if let RData::CNAME(CNAME(name)) = record.rdata
                                    && let Some(bytes) = name.as_bytes().next()
                                    && let Ok(endpoint_str) = str::from_utf8(bytes)
                                {
                                    EndpointId::from_str(endpoint_str).ok()
                                } else {
                                    None
                                }
                            },
                        ));
                    }
                }
            })
            .detach();
    }
}
