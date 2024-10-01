use bevy::{
    math::{Quat, Vec3},
    prelude::{Changed, Query, ResMut, Transform},
};

use crate::{
    ecs::components::Player,
    server::{channel::DefaultChannel, message_out::MessageOut, server::DenariaServer},
};

// Gets the Position component of all Entities whose Velocity has changed since the last run of the System
pub fn on_transform_change(
    query: Query<(&Player, &Transform), Changed<Transform>>,
    mut server: ResMut<DenariaServer>,
) {
    let mut positions: Vec<(Vec3, String)> = vec![];
    let mut rotations: Vec<(Quat, String)> = vec![];

    for (player, transform) in &query {
        positions.push((transform.translation, player.id.clone()));
        rotations.push((transform.rotation, player.id.clone()));
    }
    if positions.len() > 0 {
        if let Some(position_event) = MessageOut::position_message(positions) {
            server.broadcast_message(DefaultChannel::Unreliable, position_event.data);
        }
        if let Some(rotation_message) = MessageOut::rotation_message(rotations) {
            server.broadcast_message(DefaultChannel::Unreliable, rotation_message.data);
        }
    }
}
