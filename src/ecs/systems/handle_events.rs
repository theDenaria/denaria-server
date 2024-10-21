use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::{
    constants::{GRAVITY, JUMP_SPEED, VELOCITY_MUL},
    ecs::{
        components::{MoveInput, Player, PlayerBundle, PlayerLookup, VerticalVelocity},
        events::{ConnectEvent, DisconnectEvent, LookEvent},
    },
    server::{channel::DefaultChannel, message_out::MessageOut, server::MattaServer},
};

pub fn handle_character_movement(
    time: Res<Time>,
    mut query: Query<(
        &mut KinematicCharacterController,
        &mut MoveInput,
        &mut VerticalVelocity,
        Option<&KinematicCharacterControllerOutput>,
    )>,
) {
    let delta_time = time.delta_seconds();
    for (mut controller, mut move_input, mut v_velocity, output) in query.iter_mut() {
        let mut movement = Vec3::new(move_input.x, 0.0, move_input.z) * VELOCITY_MUL;

        if let Some(output) = output {
            if output.grounded {
                v_velocity.0 = move_input.y * JUMP_SPEED;
            } else {
                v_velocity.0 -= GRAVITY * delta_time * controller.custom_mass.unwrap_or(1.0);
            }
        }

        move_input.x = 0.0;
        move_input.y = 0.0;
        move_input.z = 0.0;

        movement.y = v_velocity.0;

        if movement != Vec3::ZERO {
            controller.translation = Some(movement);
        }
    }
}

pub fn handle_look_events(
    mut look_events: EventReader<LookEvent>,
    mut query: Query<&mut Transform>,
) {
    for event in look_events.read() {
        if let Ok(mut transform) = query.get_mut(event.entity) {
            transform.rotation = Quat::from_vec4(event.direction);
        }
    }
}

pub fn handle_spawn_events(
    mut commands: Commands,
    mut connect_events: EventReader<ConnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
) {
    for event in connect_events.read() {
        if !player_lookup.map.contains_key(&event.player_id) {
            let initial_translation = Vec3::new(25.0, 20.0, -10.0);
            let entity = commands
                .spawn(PlayerBundle {
                    player: Player {
                        id: event.player_id.clone(),
                    },
                    ..Default::default()
                })
                .insert(RigidBody::KinematicPositionBased)
                .insert(LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z)
                .insert(Collider::capsule_y(0.5, 0.5))
                .insert(ActiveCollisionTypes::default() | ActiveCollisionTypes::KINEMATIC_STATIC)
                .insert(TransformBundle::from(Transform::from_translation(
                    initial_translation,
                )))
                .insert(KinematicCharacterController {
                    offset: CharacterLength::Absolute(0.01),
                    ..KinematicCharacterController::default()
                })
                .id();

            player_lookup.map.insert(event.player_id.clone(), entity);
        }
    }
}

pub fn handle_disconnect_events(
    mut commands: Commands,
    mut disconnect_events: EventReader<DisconnectEvent>,
    mut player_lookup: ResMut<PlayerLookup>,
    mut server: ResMut<MattaServer>,
) {
    if disconnect_events.len() > 0 {
        let mut disconnect_player_ids: Vec<&String> = vec![];
        for event in disconnect_events.read() {
            if let Some(entity) = player_lookup.map.get(&event.player_id) {
                commands.entity(*entity).despawn();
                player_lookup.map.remove(&event.player_id);
                disconnect_player_ids.push(&event.player_id);
            }
        }
        let disconnect_event = MessageOut::disconnect_message(disconnect_player_ids).unwrap();
        tracing::trace!("Disconnect event: {:?}", disconnect_event);
        server.broadcast_message(DefaultChannel::ReliableOrdered, disconnect_event.data);
    }
}
