use bevy::prelude::*;

#[derive(Debug, Event)]
pub struct MoveEvent {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Event)]
pub struct LookEvent {
    pub entity: Entity,
    pub direction: Vec4,
}
#[derive(Event)]
pub struct JumpEvent {
    pub entity: Entity,
}

#[derive(Event)]
pub struct SpawnEvent {
    pub player_id: String,
}

#[derive(Event)]
pub struct DisconnectEvent {
    pub player_id: String,
}
