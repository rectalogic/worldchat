use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task, futures_lite::future},
};
use iroh::{Endpoint, RelayMode, SecretKey, protocol::Router};
use iroh_gossip::{Gossip, net::GOSSIP_ALPN};

pub fn plugin(app: &mut App) {
    app.add_observer(local_user_added)
        .add_systems(Update, poll_local_user_loading);
}

#[derive(Component)]
pub struct ChatLocalUser(UserState);

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

impl ChatLocalUser {
    pub fn new(secret_key: SecretKey) -> Self {
        Self(UserState::Loading {
            secret_key,
            task: None,
        })
    }
}

async fn load_local_user(secret_key: SecretKey) -> Result<UserState, BevyError> {
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

fn local_user_added(chat_local_user: On<Add, ChatLocalUser>, mut query: Query<&mut ChatLocalUser>) {
    if let Ok(mut chat_user) = query.get_mut(chat_local_user.entity)
        && let UserState::Loading {
            ref secret_key,
            ref mut task,
        } = chat_user.0
    {
        let secret_key = secret_key.clone();
        *task = Some(IoTaskPool::get().spawn(async move { load_local_user(secret_key).await }));
    }
}

fn poll_local_user_loading(mut chat_local_user: Single<&mut ChatLocalUser>) {
    if let UserState::Loading {
        task: Some(ref mut task),
        ..
    } = chat_local_user.0
        && let Some(result) = future::block_on(future::poll_once(task))
    {
        match result {
            Ok(user_state) => chat_local_user.0 = user_state,
            Err(e) => error!("Failed to initialize chat user: {e:?}"),
        }
    }
}
