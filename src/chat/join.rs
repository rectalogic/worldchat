use bevy::{
    prelude::*,
    tasks::{
        IoTaskPool, Task,
        futures_lite::{self, StreamExt, stream},
    },
};
use iroh::SecretKey;
use iroh_gossip::Gossip;
use pkarr::Client;

use super::{
    message::{ChatMessage, GossipEvent, SignedMessage},
    room::{ChatRoom, RoomTopic},
    user::User,
};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (join_room, handle_gossip_event));
}

#[derive(EntityEvent, Debug)]
pub struct ChatRoomEvent {
    entity: Entity,
    pub event: GossipEvent,
}

#[derive(Component)]
pub struct RoomConnection {
    tx: async_channel::Sender<ChatMessage>,
    rx: async_channel::Receiver<GossipEvent>,
    _task: Task<()>,
}

impl RoomConnection {
    pub fn send(&self, message: ChatMessage) -> Result<(), BevyError> {
        self.tx.try_send(message)?;
        Ok(())
    }
}

fn join_room(
    mut commands: Commands,
    query: Query<(Entity, &ChatRoom), Without<RoomConnection>>,
    user: Single<&User>,
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
            _task: IoTaskPool::get().spawn({
                async move {
                    if let Err(e) =
                        room_event_loop(topic, secret_key, gossip, bevy_rx, gossip_tx).await
                    {
                        error!("Failed to join chat room: {e:?}");
                    }
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
    gossip_tx: async_channel::Sender<GossipEvent>,
) -> Result<(), BevyError> {
    let client = Client::builder().build()?;
    let gossip_topic = gossip
        .subscribe(
            topic.topic_id(),
            topic
                .resolve_bootstrap_endpoint_ids(&client, secret_key.public())
                .await,
        )
        .await?;

    let endpoint_id = secret_key.public();
    let _endpoint_publisher_task = IoTaskPool::get().spawn(async move {
        if let Err(e) = topic.publish_endpoint_cname(&client, endpoint_id, 15).await {
            error!("Endpoint CNAME publisher failed: {e:?}")
        }
    });

    let (gossip_sender, gossip_receiver) = gossip_topic.split();
    let events = stream::race(
        gossip_receiver.map(StreamItem::GossipEvent),
        bevy_rx.map(StreamItem::ChatMessage),
    );
    futures_lite::pin!(events);

    while let Some(e) = events.next().await {
        match e {
            StreamItem::ChatMessage(message) => {
                let signed_message = SignedMessage::sign_and_encode(&secret_key, message)?;
                if let Err(e) = gossip_sender.broadcast(signed_message.into()).await {
                    error!("Failed to send message: {e:?}")
                }
            }
            StreamItem::GossipEvent(result) => match result {
                Ok(event) => {
                    let gossip_event: GossipEvent = match event.try_into() {
                        Ok(event) => event,
                        Err(err) => {
                            warn!("received invalid message: {err}");
                            continue;
                        }
                    };
                    if let Err(e) = gossip_tx.send(gossip_event).await {
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
