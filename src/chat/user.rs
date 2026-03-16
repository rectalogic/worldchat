use bevy::prelude::*;
use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task, futures_lite::future},
};
use iroh::{Endpoint, RelayMode, SecretKey, protocol::Router};
use iroh_gossip::{Gossip, net::GOSSIP_ALPN};
use std::ops::DerefMut;

pub fn plugin(app: &mut App) {
    app.add_observer(user_loader_added)
        .add_systems(Update, poll_loader_task);
}

#[derive(Component)]
pub struct ChatUserLoader {
    secret_key: SecretKey,
    task: Option<Task<Result<ChatUser, BevyError>>>,
}

impl ChatUserLoader {
    pub fn new(secret_key: SecretKey) -> Self {
        Self {
            secret_key,
            task: None,
        }
    }
}

#[derive(Component)]
pub struct ChatUser {
    endpoint: Endpoint,
    router: Router,
    gossip: Gossip,
}

impl ChatUser {
    async fn new(secret_key: SecretKey) -> Result<Self, BevyError> {
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .relay_mode(RelayMode::Default)
            .bind()
            .await?;
        let gossip = Gossip::builder().spawn(endpoint.clone());
        // Wait for home relay
        endpoint.online().await;
        let router = Router::builder(endpoint.clone())
            .accept(GOSSIP_ALPN, gossip.clone())
            .spawn();
        Ok(Self {
            endpoint,
            router,
            gossip,
        })
    }
}

fn user_loader_added(user_loader: On<Add, ChatUserLoader>, mut query: Query<&mut ChatUserLoader>) {
    if let Ok(mut loader) = query.get_mut(user_loader.entity) {
        let secret_key = loader.secret_key.clone();
        loader.task = Some(IoTaskPool::get().spawn(async move { ChatUser::new(secret_key).await }));
    }
}

fn poll_loader_task(mut commands: Commands, mut loader: Single<(Entity, &mut ChatUserLoader)>) {
    let (entity, loader) = loader.deref_mut();
    if let Some(ref mut task) = loader.task
        && let Some(result) = future::block_on(future::poll_once(task))
    {
        match result {
            Ok(user) => {
                commands
                    .entity(*entity)
                    .insert(user)
                    .remove::<ChatUserLoader>();
            }
            Err(e) => error!("Failed to initialize chat user: {e:?}"),
        }
    }
}
