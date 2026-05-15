use bevy::prelude::*;

#[derive(Component, Default, Debug)]
#[require(User)]
pub struct PrimaryUser;

#[derive(Component, Default, Debug)]
#[require(Transform)]
pub struct User;

#[derive(EntityEvent, Debug)]
pub struct UserMessage {
    #[event_target]
    pub user_entity: Entity,
    pub message: String,
}

#[derive(Event)]
pub struct UserJoined;

#[derive(Event)]
pub struct PrimaryUserNameSet(pub String);
