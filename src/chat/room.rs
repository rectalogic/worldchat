use std::{str::FromStr, time::Duration};

use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use iroh::{Endpoint, EndpointId, SecretKey};
use iroh_gossip::{Gossip, TopicId};
use pkarr::{
    Client,
    dns::rdata::{CNAME, RData},
};
use rand::seq::IndexedRandom;

pub fn plugin(app: &mut App) {
    // app.add_systems(Update, publish_endpoint_id);
}

#[derive(Component)]
pub struct ChatRoom {
    name: String,
    topic_id: TopicId,
    well_known_peer_cnames: Vec<pkarr::PublicKey>,
    membership_task: Option<Task<()>>,
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
            membership_task: None,
        }
    }

    fn join(&mut self, secret_key: SecretKey, gossip: Gossip) {
        let topic_id = self.topic_id;
        let well_known_peer_cnames = self.well_known_peer_cnames.clone();
        self.membership_task = Some(IoTaskPool::get().spawn(async move {
            if let Err(e) = join_room(topic_id, well_known_peer_cnames, secret_key, gossip).await {
                error!("Failed to join chat room: {e:?}");
            }
        }));
    }

    fn leave(&mut self) {
        self.membership_task = None;
    }
}

async fn join_room(
    topic_id: TopicId,
    well_known_peer_cnames: Vec<pkarr::PublicKey>,
    secret_key: SecretKey,
    gossip: Gossip,
) -> Result<(), BevyError> {
    let client = Client::builder().build()?;
    let gossip_topic = gossip
        .subscribe_and_join(
            topic_id,
            resolve_bootstrap_endpoint_ids(&client, &well_known_peer_cnames).await,
        )
        .await?;

    let endpoint_cname = well_known_peer_cnames
        .choose(&mut rand::rng())
        .unwrap()
        .clone();
    let _endpoint_publisher_task = IoTaskPool::get().spawn(async move {
        if let Err(e) = publish_endpoint_id(client, endpoint_cname, secret_key).await {
            error!("Endpoint CNAME publisher failed: {e:?}")
        }
    });

    //XXX need 2 async_channels, for msg from bevy and msg from iroh, use race() or future::select() to wait on them
    // from iroh is a Stream - async_channel::Receiver implements Stream, so we can use select on 2 streams

    Ok(())
}

async fn publish_endpoint_id(
    client: Client,
    cname: pkarr::PublicKey,
    secret_key: SecretKey,
) -> Result<(), BevyError> {
    let ttl = 15u32;
    let delay = Duration::from_secs(ttl as u64);

    let keypair = pkarr::Keypair::from_secret_key(&secret_key.to_bytes());
    let signed_packet = pkarr::SignedPacket::builder()
        .cname(
            pkarr::dns::Name::new(&cname.to_string())?,
            pkarr::dns::Name::new(&secret_key.public().to_string())?,
            ttl,
        )
        .build(&keypair)?;

    loop {
        client.publish(&signed_packet, None).await?;
        futures_timer::Delay::new(delay).await;
    }
}

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
