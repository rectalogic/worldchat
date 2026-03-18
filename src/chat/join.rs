use std::{str::FromStr, time::Duration};

use bevy::{
    prelude::*,
    tasks::futures_lite::{self, stream},
};
use iroh::{EndpointId, SecretKey, endpoint_info::EndpointIdExt};
use iroh_gossip::{Gossip, TopicId};
use pkarr::{
    Client,
    dns::rdata::{CNAME, RData},
};
use rand::seq::IndexedRandom;
use tokio_stream::StreamExt;

use crate::tokio::{Task, TokioRuntime};

use super::{room::ChatRoom, user::User};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (join_room, handle_gossip_event));
}

#[derive(EntityEvent, Debug)]
pub struct ChatRoomEvent {
    entity: Entity,
    pub event: iroh_gossip::api::Event,
}

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
    query: Query<(Entity, &ChatRoom), Without<RoomConnection>>,
    user: Single<&User>,
    tokio: Res<TokioRuntime>,
) {
    for (room_entity, room) in query {
        let topic_id = room.topic_id();
        let bootstrap_keypairs = room.bootstrap_keypairs().to_vec();
        let secret_key = user.endpoint().secret_key().clone();
        let gossip = user.gossip().clone();
        let (bevy_tx, bevy_rx) = async_channel::unbounded();
        let (gossip_tx, gossip_rx) = async_channel::unbounded();
        commands.entity(room_entity).insert(RoomConnection {
            tx: bevy_tx,
            rx: gossip_rx,
            task: Task::spawn(&tokio, async move {
                if let Err(e) = join_room_task(
                    topic_id,
                    bootstrap_keypairs,
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
    bootstrap_keypairs: Vec<pkarr::Keypair>,
    secret_key: SecretKey,
    gossip: Gossip,
    bevy_rx: async_channel::Receiver<String>,
    gossip_tx: async_channel::Sender<iroh_gossip::api::Event>,
) -> Result<(), BevyError> {
    let client = Client::builder().build()?;
    let (gossip_sender, gossip_receiver) = gossip
        .subscribe(
            topic_id,
            resolve_bootstrap_endpoint_ids(&client, &bootstrap_keypairs).await,
        )
        .await?
        .split();

    let cname_keypair = bootstrap_keypairs.choose(&mut rand::rng()).unwrap().clone();
    let _endpoint_publisher_task = Task::new(tokio::task::spawn(async move {
        if let Err(e) = publish_endpoint_id(client, cname_keypair, secret_key.public()).await {
            error!("Endpoint CNAME publisher failed: {e:?}")
        }
    }));

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
    cname_keypair: pkarr::Keypair,
    endpoint_id: EndpointId,
) -> Result<(), BevyError> {
    let ttl = 15u32;
    let delay = Duration::from_secs(ttl as u64);

    let signed_packet = pkarr::SignedPacket::builder()
        .cname(
            pkarr::dns::Name::new(&cname_keypair.public_key().to_z32())?,
            pkarr::dns::Name::new(&endpoint_id.to_z32())?,
            ttl,
        )
        .build(&cname_keypair)?;

    loop {
        if let Err(e) = client.publish(&signed_packet, None).await {
            warn!("Failed to publish CNAME: {e:?}");
        }
        futures_timer::Delay::new(delay).await;
    }
}

// Resolve pkarr::PublicKey CNAMES to their target EndpointIds
async fn resolve_bootstrap_endpoint_ids(
    client: &Client,
    bootstrap_keypairs: &[pkarr::Keypair],
) -> Vec<EndpointId> {
    let mut bootstrap_endpoint_ids = Vec::new();
    for keypair in bootstrap_keypairs {
        if let Some(packet) = client.resolve_most_recent(&keypair.public_key()).await {
            bootstrap_endpoint_ids.extend(packet.all_resource_records().filter_map(|record| {
                if let RData::CNAME(CNAME(name)) = &record.rdata
                    && let Some(bytes) = name.as_bytes().next()
                    && let Ok(endpoint_str) = str::from_utf8(bytes)
                {
                    EndpointId::from_z32(endpoint_str).ok()
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
