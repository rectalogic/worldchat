use bevy::{ecs::relationship::Relationship, prelude::*};

mod channel;
mod message;
mod server;
mod user;

pub use channel::{ChannelOfServer, ChannelUsers, UserAdded};
pub use message::IrcControl;
pub use server::{Server, ServerChannels, UserNameChanged};
pub use user::{PrimaryUser, UserMessage, UserOfChannel};

pub struct IrcPlugin;

impl Plugin for IrcPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(server::ServerPlugin);
    }
}

pub fn find_relationship_source_named<RS: Relationship, RT: RelationshipTarget>(
    source_name: &Name,
    target_entity: Entity,
    targets: Query<&RT>,
    sources: Query<(Entity, &Name), With<RS>>,
) -> Option<Entity> {
    targets.relationship_sources::<RT>(target_entity).find(
        |source_entity| matches!(sources.get(*source_entity), Ok((_, name)) if name == source_name),
    )
}
