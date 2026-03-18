use std::{str::FromStr, time::Duration};

use bevy::{
    prelude::*,
    tasks::{
        IoTaskPool, Task,
        futures_lite::{self, StreamExt, stream},
    },
};
use iroh::{EndpointId, SecretKey};
use iroh_gossip::{Gossip, TopicId};
use pkarr::{
    Client,
    dns::rdata::{CNAME, RData},
};
use rand::seq::IndexedRandom;

use super::{room::ChatRoom, user::User};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (join_room, handle_gossip_event));
}

#[derive(EntityEvent)]
pub struct ChatRoomEvent {
    entity: Entity,
    pub event: iroh_gossip::api::Event,
}

#[derive(Component)]
pub struct JoinRoomRequest;

#[derive(Component)]
pub struct RoomConnection {
    tx: async_channel::Sender<String>,
    rx: async_channel::Receiver<iroh_gossip::api::Event>,
    task: Task<()>,
}

impl RoomConnection {
    pub fn send(&self, message: String) -> Result<(), BevyError> {
        self.tx.send_blocking(message)?;
        Ok(())
    }
}

#[allow(clippy::type_complexity)]
fn join_room(
    mut commands: Commands,
    query: Query<(Entity, &ChatRoom), (With<JoinRoomRequest>, Without<RoomConnection>)>,
    user: Single<&User>,
) {
    for (room_entity, room) in query {
        let topic_id = room.topic_id();
        let bootstrap_ids = room.bootstrap_ids().to_vec();
        let secret_key = user.endpoint().secret_key().clone();
        let gossip = user.gossip().clone();
        let (bevy_tx, bevy_rx) = async_channel::unbounded();
        let (gossip_tx, gossip_rx) = async_channel::unbounded();
        commands.entity(room_entity).insert(RoomConnection {
            tx: bevy_tx,
            rx: gossip_rx,
            task: IoTaskPool::get().spawn(async move {
                if let Err(e) = join_room_task(
                    topic_id,
                    bootstrap_ids,
                    secret_key,
                    gossip,
                    bevy_rx,
                    gossip_tx,
                )
                .await
                {
                    error!("Failed to join chat room: {e:?}");
                }
            }),
        });
    }
}

enum StreamItem {
    BevyMessage(String),
    GossipEvent(Result<iroh_gossip::api::Event, iroh_gossip::api::ApiError>),
}

async fn join_room_task(
    topic_id: TopicId,
    bootstrap_ids: Vec<pkarr::PublicKey>,
    secret_key: SecretKey,
    gossip: Gossip,
    bevy_rx: async_channel::Receiver<String>,
    gossip_tx: async_channel::Sender<iroh_gossip::api::Event>,
) -> Result<(), BevyError> {
    let client = Client::builder().build()?;
    let (gossip_sender, gossip_receiver) = gossip
        .subscribe_and_join(
            topic_id,
            resolve_bootstrap_endpoint_ids(&client, &bootstrap_ids).await,
        )
        .await?
        .split();

    let endpoint_cname = bootstrap_ids.choose(&mut rand::rng()).unwrap().clone();
    let _endpoint_publisher_task = IoTaskPool::get().spawn(async move {
        if let Err(e) = publish_endpoint_id(client, endpoint_cname, secret_key).await {
            error!("Endpoint CNAME publisher failed: {e:?}")
        }
    });

    let events = stream::race(
        gossip_receiver.map(StreamItem::GossipEvent),
        bevy_rx.map(StreamItem::BevyMessage),
    );
    futures_lite::pin!(events);

    while let Some(e) = events.next().await {
        match e {
            StreamItem::BevyMessage(message) => {
                if let Err(e) = gossip_sender.broadcast(message.into()).await {
                    error!("Failed to send message: {e:?}")
                }
            }
            StreamItem::GossipEvent(result) => match result {
                Ok(event) => {
                    if let Err(e) = gossip_tx.send(event).await {
                        error!("Failed to send Gossip event to Bevy: {e:?}");
                    }
                }
                Err(e) => {
                    error!("Received Gossip API error: {e:?}");
                }
            },
        }
    }

    Ok(())
}

// Periodically publish a pkarr::PublicKey CNAME to our EndpointId
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

// Resolve pkarr::PublicKey CNAMES to their target EndpointIds
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

fn handle_gossip_event(mut commands: Commands, room_connections: Query<(Entity, &RoomConnection)>) {
    for (room_entity, room_connection) in room_connections {
        while let Ok(event) = room_connection.rx.try_recv() {
            commands.trigger(ChatRoomEvent {
                entity: room_entity,
                event,
            });
        }
    }
}
