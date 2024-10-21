use bevy::{
    math::{Quat, Vec3},
    prelude::{Added, Changed, Query, ResMut, Transform},
};

use crate::{
    ecs::components::Player,
    server::{channel::DefaultChannel, message_out::MessageOut, server::MattaServer},
};

// Gets the Position component of all Entities whose Velocity has changed since the last run of the System
pub fn on_transform_change(
    query: Query<(&Player, &Transform), Changed<Transform>>,
    mut server: ResMut<MattaServer>,
) {
    let mut positions: Vec<(Vec3, String)> = vec![];
    let mut rotations: Vec<(Quat, String)> = vec![];

    for (player, transform) in &query {
        positions.push((transform.translation, player.id.clone()));
        rotations.push((transform.rotation, player.id.clone()));
    }
    if positions.len() > 0 {
        if let Some(position_event) = MessageOut::position_message(positions.clone()) {
            server.broadcast_message(DefaultChannel::Unreliable, position_event.data);
        }
        if let Some(rotation_message) = MessageOut::rotation_message(rotations) {
            server.broadcast_message(DefaultChannel::Unreliable, rotation_message.data);
        }
    }
}

pub fn on_spawn_change(
    query: Query<(&Player, &Transform), Added<Transform>>,
    mut server: ResMut<MattaServer>,
) {
    for (player, transform) in &query {
        if let Some(spawn_message) =
            MessageOut::spawn_message(player.id.clone(), transform.translation, transform.rotation)
        {
            server.broadcast_message(DefaultChannel::ReliableOrdered, spawn_message.data);
        }
    }
}
