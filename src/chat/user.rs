use bevy::prelude::*;
use iroh::{Endpoint, SecretKey, endpoint::presets, protocol::Router};
use iroh_gossip::{Gossip, net::GOSSIP_ALPN};

use crate::tokio::{Task, TokioRuntime};

pub struct UserPlugin {
    pub secret_key: SecretKey,
}

impl Plugin for UserPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, poll_user_loading)
            .add_systems(Last, shutdown_endpoint);
        let secret_key = self.secret_key.clone();
        app.world_mut().spawn(UserLoader {
            task: Task::new(tokio::task::spawn(
                async move { load_user(secret_key).await },
            )),
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

fn shutdown_endpoint(
    user: Single<&User>,
    tokio: Res<TokioRuntime>,
    exit_reader: MessageReader<AppExit>,
) {
    if !exit_reader.is_empty() {
        tokio.block_on(user.endpoint().close());
    }
}

async fn load_user(secret_key: SecretKey) -> Result<User, BevyError> {
    let endpoint = Endpoint::builder(presets::N0)
        .secret_key(secret_key)
        .alpns(vec![GOSSIP_ALPN.to_vec()])
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

fn poll_user_loading(
    mut commands: Commands,
    mut user: Single<(Entity, &mut UserLoader)>,
    tokio: Res<TokioRuntime>,
) {
    let (entity, ref mut user) = *user;
    if let Some(result) = user.task.result(&tokio) {
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
