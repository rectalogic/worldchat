use bevy::{
    prelude::*,
    tasks::{IoTaskPool, Task, futures_lite::future},
};
use iroh::{Endpoint, RelayMode, SecretKey, protocol::Router};
use iroh_gossip::{Gossip, net::GOSSIP_ALPN};

mod room;

pub struct ChatPlugin {
    pub secret_key: SecretKey,
}

impl Plugin for ChatPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(room::plugin)
            .insert_resource(ChatLoaderTask::new(self.secret_key.clone()))
            .add_systems(Update, poll_loader_task);
    }
}

#[derive(Resource)]
struct ChatLoaderTask(Task<Result<Chat, BevyError>>);

impl ChatLoaderTask {
    fn new(secret_key: SecretKey) -> Self {
        Self(IoTaskPool::get().spawn(async move { Chat::new(secret_key).await }))
    }
}

fn poll_loader_task(mut commands: Commands, task: Option<ResMut<ChatLoaderTask>>) {
    if let Some(mut task) = task
        && let Some(result) = future::block_on(future::poll_once(&mut task.0))
    {
        match result {
            Ok(client) => {
                commands.insert_resource(client);
                commands.remove_resource::<ChatLoaderTask>();
            }
            Err(e) => error!("Failed to initialize chat: {e:?}"),
        }
    }
}

#[derive(Resource)]
pub struct Chat {
    endpoint: Endpoint,
    router: Router,
    gossip: Gossip,
}

impl Chat {
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
