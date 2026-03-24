use std::time::Duration;

use bevy::prelude::*;
use futures_timer::Delay;
use iroh::{EndpointId, endpoint_info::EndpointIdExt};
use iroh_gossip::TopicId;
use pkarr::dns::rdata::{CNAME, RData};

use crate::chat::to_z32;

const MAX_BOOTSTRAP_RECORDS: u32 = 20;

#[derive(Component)]
pub struct ChatRoom {
    topic: RoomTopic,
}

impl ChatRoom {
    pub fn new(keypair: pkarr::Keypair) -> Self {
        Self {
            topic: RoomTopic::new(keypair),
        }
    }

    pub fn topic(&self) -> &RoomTopic {
        &self.topic
    }
}

#[derive(Clone)]
pub struct RoomTopic {
    dns_publisher_keypair: pkarr::Keypair,
    topic_id: TopicId,
}

impl RoomTopic {
    fn new(keypair: pkarr::Keypair) -> Self {
        let public_key = keypair.public_key();
        let public_bytes = public_key.as_bytes();
        Self {
            topic_id: TopicId::from_bytes(*public_bytes),
            dns_publisher_keypair: keypair,
        }
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    // Resolve room CNAME to target EndpointIds
    pub async fn resolve_bootstrap_endpoint_ids(
        &self,
        client: &pkarr::Client,
        endpoint_id: EndpointId,
    ) -> Vec<EndpointId> {
        let mut bootstrap_endpoint_ids = Vec::new();

        if let Some(packet) = client
            .resolve_most_recent(&self.dns_publisher_keypair.public_key())
            .await
        {
            let mut count = 0;
            bootstrap_endpoint_ids.extend(packet.fresh_resource_records(".").filter_map(
                |record| {
                    if let RData::CNAME(CNAME(name)) = &record.rdata
                        && let Some(bytes) = name.as_bytes().next()
                        && bytes != endpoint_id.as_bytes()
                        && let Ok(endpoint_str) = str::from_utf8(bytes)
                    {
                        count += 1;
                        if count > MAX_BOOTSTRAP_RECORDS {
                            None
                        } else {
                            EndpointId::from_z32(endpoint_str).ok()
                        }
                    } else {
                        None
                    }
                },
            ));
        }

        bootstrap_endpoint_ids
    }

    // Periodically publish our EndpointId to the rooms DNS CNAME
    pub async fn publish_endpoint_cname(
        &self,
        client: &pkarr::Client,
        endpoint_id: EndpointId,
        ttl: u32,
    ) -> Result<(), BevyError> {
        let delay = Duration::from_secs((ttl as f64 * 0.75) as u64);
        let endpoint_z32 = endpoint_id.to_z32();
        let endpoint_name = pkarr::dns::Name::new(&endpoint_z32)?;

        loop {
            let mut builder = pkarr::SignedPacket::builder();
            if let Some(most_recent) = client
                .resolve_most_recent(&self.dns_publisher_keypair.public_key())
                .await
            {
                for record in most_recent.fresh_resource_records(".") {
                    match record.rdata {
                        RData::CNAME(ref cname) if cname.0 != endpoint_name => {
                            match record.ttl.overflowing_sub(most_recent.elapsed()) {
                                (_, true) => {}
                                (ttl, false) => {
                                    let mut record = record.clone();
                                    record.ttl = ttl;
                                    builder = builder.record(record);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            };

            let signed_packet = builder
                .cname(".".try_into()?, endpoint_name.clone(), ttl)
                .sign(&self.dns_publisher_keypair)?;

            if let Err(e) = client.publish(&signed_packet, None).await {
                warn!("Failed to publish CNAME: {e:?}");
                Delay::new(delay / 3).await
            } else {
                Delay::new(delay).await
            }
        }
    }
}
