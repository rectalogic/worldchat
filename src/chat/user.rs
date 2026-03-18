use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task, futures_lite::future},
};
use iroh::{Endpoint, RelayMode, SecretKey, protocol::Router};
use iroh_gossip::{Gossip, net::GOSSIP_ALPN};

pub struct UserPlugin {
    pub secret_key: SecretKey,
}

impl Plugin for UserPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, poll_user_loading);
        let secret_key = self.secret_key.clone();
        app.world_mut().spawn(UserLoader {
            task: IoTaskPool::get().spawn(async move { load_user(secret_key).await }),
        });
    }
}

#[derive(Component)]
pub struct UserLoader {
    task: Task<Result<User, BevyError>>,
}

#[derive(Component)]
pub struct User {
    endpoint: Endpoint,
    router: Router,
    gossip: Gossip,
}

impl User {
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn gossip(&self) -> &Gossip {
        &self.gossip
    }
}

async fn load_user(secret_key: SecretKey) -> Result<User, BevyError> {
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
    Ok(User {
        endpoint,
        router,
        gossip,
    })
}

fn poll_user_loading(mut commands: Commands, mut user: Single<(Entity, &mut UserLoader)>) {
    let (entity, ref mut user) = *user;
    if let Some(result) = future::block_on(future::poll_once(&mut user.task)) {
        match result {
            Ok(loaded_user) => {
                commands
                    .entity(entity)
                    .insert(loaded_user)
                    .remove::<UserLoader>();
            }
            Err(e) => error!("Failed to initialize chat user: {e:?}"),
        }
    }
}
