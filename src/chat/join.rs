use std::{str::FromStr, time::Duration};

use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task},
};
use iroh::{EndpointId, SecretKey};
use iroh_gossip::{Gossip, TopicId};
use pkarr::{
    Client,
    dns::rdata::{CNAME, RData},
};
use rand::seq::IndexedRandom;

use super::{room::Room, user::LoadedUser};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, join_room);
}

#[derive(Component)]
pub struct JoinRequest;

#[derive(Component)]
pub struct Joined {
    // rx: async_channel::Receiver<String>,
    task: Task<()>,
}

#[allow(clippy::type_complexity)]
fn join_room(
    mut commands: Commands,
    query: Query<(Entity, &Room), (With<JoinRequest>, Without<Joined>)>,
    user: Single<&LoadedUser>,
) {
    for (room_entity, room) in query {
        let topic_id = room.topic_id();
        let bootstrap_ids = room.bootstrap_ids().to_vec();
        let secret_key = user.endpoint().secret_key().clone();
        let gossip = user.gossip().clone();
        commands.entity(room_entity).insert(Joined {
            task: IoTaskPool::get().spawn(async move {
                if let Err(e) = join_room_task(topic_id, bootstrap_ids, secret_key, gossip).await {
                    error!("Failed to join chat room: {e:?}");
                }
            }),
        });
    }
}

async fn join_room_task(
    topic_id: TopicId,
    bootstrap_ids: Vec<pkarr::PublicKey>,
    secret_key: SecretKey,
    gossip: Gossip,
) -> Result<(), BevyError> {
    let client = Client::builder().build()?;
    let gossip_topic = gossip
        .subscribe_and_join(
            topic_id,
            resolve_bootstrap_endpoint_ids(&client, &bootstrap_ids).await,
        )
        .await?;

    let endpoint_cname = bootstrap_ids.choose(&mut rand::rng()).unwrap().clone();
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
    bootstrap_ids: &[pkarr::PublicKey],
) -> Vec<EndpointId> {
    let mut bootstrap_endpoint_ids = Vec::new();
    for ep in bootstrap_ids {
        if let Some(packet) = client.resolve_most_recent(ep).await {
            bootstrap_endpoint_ids.extend(packet.all_resource_records().filter_map(|record| {
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
    bootstrap_endpoint_ids
}
