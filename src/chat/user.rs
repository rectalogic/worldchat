use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task, futures_lite::future},
};
use iroh::{Endpoint, RelayMode, SecretKey, protocol::Router};
use iroh_gossip::{Gossip, net::GOSSIP_ALPN};

pub fn plugin(app: &mut App) {
    app.add_observer(user_added)
        .add_systems(Update, poll_user_loading);
}

#[derive(Component)]
pub struct User {
    secret_key: SecretKey,
    task: Option<Task<Result<LoadedUser, BevyError>>>,
}

#[derive(Component)]
pub struct LoadedUser {
    endpoint: Endpoint,
    router: Router,
    gossip: Gossip,
}

impl User {
    pub fn new(secret_key: SecretKey) -> Self {
        Self {
            secret_key,
            task: None,
        }
    }
}

impl LoadedUser {
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }
}

async fn load_user(secret_key: SecretKey) -> Result<LoadedUser, BevyError> {
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
    Ok(LoadedUser {
        endpoint,
        router,
        gossip,
    })
}

fn user_added(user: On<Add, User>, mut query: Query<&mut User, Without<LoadedUser>>) {
    if let Ok(mut user) = query.get_mut(user.entity) {
        let secret_key = user.secret_key.clone();
        user.task = Some(IoTaskPool::get().spawn(async move { load_user(secret_key).await }));
    }
}

fn poll_user_loading(mut commands: Commands, mut user: Single<(Entity, &mut User)>) {
    let (entity, ref mut user) = *user;
    if let Some(ref mut task) = user.task
        && let Some(result) = future::block_on(future::poll_once(task))
    {
        match result {
            Ok(loaded_user) => {
                user.task = None;
                commands.entity(entity).insert(loaded_user);
            }
            Err(e) => error!("Failed to initialize chat user: {e:?}"),
        }
    }
}
