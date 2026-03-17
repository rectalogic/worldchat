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
pub struct User(UserState);

enum UserState {
    Loading {
        secret_key: SecretKey,
        task: Option<Task<Result<UserState, BevyError>>>,
    },
    Loaded {
        endpoint: Endpoint,
        router: Router,
        gossip: Gossip,
    },
}

impl User {
    pub fn new(secret_key: SecretKey) -> Self {
        Self(UserState::Loading {
            secret_key,
            task: None,
        })
    }
}

async fn load_user(secret_key: SecretKey) -> Result<UserState, BevyError> {
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
    Ok(UserState::Loaded {
        endpoint,
        router,
        gossip,
    })
}

fn user_added(user: On<Add, User>, mut query: Query<&mut User>) {
    if let Ok(mut user) = query.get_mut(user.entity)
        && let UserState::Loading {
            ref secret_key,
            ref mut task,
        } = user.0
    {
        let secret_key = secret_key.clone();
        *task = Some(IoTaskPool::get().spawn(async move { load_user(secret_key).await }));
    }
}

fn poll_user_loading(mut user: Single<&mut User>) {
    if let UserState::Loading {
        task: Some(ref mut task),
        ..
    } = user.0
        && let Some(result) = future::block_on(future::poll_once(task))
    {
        match result {
            Ok(user_state) => user.0 = user_state,
            Err(e) => error!("Failed to initialize chat user: {e:?}"),
        }
    }
}
