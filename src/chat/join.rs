use std::time::Duration;

use bevy::{
    prelude::*,
    tasks::futures_lite::{self, stream},
};
use iroh::{EndpointId, SecretKey, endpoint_info::EndpointIdExt};
use iroh_gossip::Gossip;
use pkarr::{Client, dns::rdata::RData};
use tokio_stream::StreamExt;

use crate::{
    chat::room::RoomTopic,
    tokio::{Task, TokioRuntime},
};

use super::{room::ChatRoom, user::User};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (join_room, handle_gossip_event));
}

#[derive(EntityEvent, Debug)]
pub struct ChatRoomEvent {
    entity: Entity,
    pub event: iroh_gossip::api::Event,
}

#[derive(Debug)]
pub enum ChatMessage {
    Message(String),
}

#[derive(Component)]
pub struct RoomConnection {
    tx: async_channel::Sender<ChatMessage>,
    rx: async_channel::Receiver<iroh_gossip::api::Event>,
    task: Task<()>,
}

impl RoomConnection {
    pub fn send(&self, message: ChatMessage) -> Result<(), BevyError> {
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
        let secret_key = user.endpoint().secret_key().clone();
        let topic = room.topic().clone();
        let gossip = user.gossip().clone();
        let (bevy_tx, bevy_rx) = async_channel::unbounded();
        let (gossip_tx, gossip_rx) = async_channel::unbounded();
        commands.entity(room_entity).insert(RoomConnection {
            tx: bevy_tx,
            rx: gossip_rx,
            task: Task::spawn(&tokio, async move {
                if let Err(e) = room_event_loop(topic, secret_key, gossip, bevy_rx, gossip_tx).await
                {
                    error!("Failed to join chat room: {e:?}");
                }
            }),
        });
    }
}

enum StreamItem {
    ChatMessage(ChatMessage),
    GossipEvent(Result<iroh_gossip::api::Event, iroh_gossip::api::ApiError>),
}

async fn room_event_loop(
    topic: RoomTopic,
    secret_key: SecretKey,
    gossip: Gossip,
    bevy_rx: async_channel::Receiver<ChatMessage>,
    gossip_tx: async_channel::Sender<iroh_gossip::api::Event>,
) -> Result<(), BevyError> {
    let client = Client::builder().build()?;
    let (gossip_sender, gossip_receiver) = gossip
        .subscribe(
            topic.topic_id(),
            topic
                .resolve_bootstrap_endpoint_ids(&client, secret_key.public())
                .await,
        )
        .await?
        .split();

    let _endpoint_publisher_task = Task::new(tokio::task::spawn(async move {
        if let Err(e) = publish_endpoint_cname(client, topic, secret_key.public(), 15).await {
            error!("Endpoint CNAME publisher failed: {e:?}")
        }
    }));

    let events = stream::race(
        gossip_receiver.map(StreamItem::GossipEvent),
        bevy_rx.map(StreamItem::ChatMessage),
    );
    futures_lite::pin!(events);

    while let Some(e) = events.next().await {
        match e {
            StreamItem::ChatMessage(message) => match message {
                ChatMessage::Message(message) => {
                    if let Err(e) = gossip_sender.broadcast(message.into()).await {
                        error!("Failed to send message: {e:?}")
                    }
                }
            },
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

// Periodically publish our EndpointId to the rooms DNS CNAME
async fn publish_endpoint_cname(
    client: Client,
    topic: RoomTopic,
    endpoint_id: EndpointId,
    ttl: u32,
) -> Result<(), BevyError> {
    let delay = Duration::from_secs((ttl as f64 * 0.75) as u64);
    let endpoint_z32 = endpoint_id.to_z32();
    let endpoint_name = pkarr::dns::Name::new(&endpoint_z32)?;

    loop {
        let mut builder = pkarr::SignedPacket::builder();
        let (builder, cas) = if let Some(most_recent) = client
            .resolve_most_recent(&topic.dns_publisher_keypair().public_key())
            .await
        {
            for record in most_recent.fresh_resource_records(topic.bootstrap_dns_name()) {
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
            (builder, Some(most_recent.timestamp()))
        } else {
            (builder, None)
        };

        let signed_packet = builder
            .cname(
                topic.bootstrap_dns_name().try_into()?,
                endpoint_name.clone(),
                ttl,
            )
            .sign(topic.dns_publisher_keypair())?;

        if let Err(e) = client.publish(&signed_packet, cas).await {
            warn!("Failed to publish CNAME: {e:?}");
        }
        tokio::time::sleep(delay).await;
    }
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
