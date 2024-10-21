use bevy::prelude::{EventWriter, Query, Res, ResMut};

use crate::{
    constants::TICK_DELTA,
    ecs::{
        components::{MoveInput, PlayerLookup},
        events::{ConnectEvent, DisconnectEvent, FireEvent, LookEvent},
        systems::send_events::{send_disconnect_event, send_look_event},
    },
    server::{
        channel::DefaultChannel,
        message_in::{digest_move_message, digest_rotation_message, MessageIn, MessageInType},
        server::{MattaServer, ServerEvent},
        transport::transport::ServerTransport,
    },
};

use super::send_events::send_connect_event;

pub fn handle_server_events(
    mut server: ResMut<MattaServer>,
    mut transport: ResMut<ServerTransport>,
    mut disconnect_event: EventWriter<DisconnectEvent>,
) {
    server.update(TICK_DELTA);
    transport.update(TICK_DELTA, &mut server).unwrap();

    // Check for client connections/disconnections
    while let Some(event) = server.get_event() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Client {client_id} connected");
            }
            ServerEvent::ClientDisconnected {
                client_id,
                player_id,
                reason,
            } => {
                println!("Client {client_id} disconnected: {reason}");
                send_disconnect_event(player_id, &mut disconnect_event);
            }
        }
    }
}

pub fn handle_server_messages(
    mut server: ResMut<MattaServer>,
    player_lookup: Res<PlayerLookup>,
    mut connect_event: EventWriter<ConnectEvent>,
    mut move_query: Query<&mut MoveInput>,
    mut look_event: EventWriter<LookEvent>,
) {
    // Receive message from channel

    server.clients_id().iter().for_each(|client_id| {
        while let Some((message, player_id)) =
            server.receive_message(*client_id, DefaultChannel::Unreliable)
        {
            let event_in = MessageIn::new(message.to_vec()).unwrap();

            match event_in.event_type {
                MessageInType::Rotation => {
                    let rotation = digest_rotation_message(event_in.data).unwrap();
                    send_look_event(
                        player_id,
                        rotation.x,
                        rotation.y,
                        rotation.z,
                        rotation.w,
                        &player_lookup,
                        &mut look_event,
                    );
                }
                MessageInType::Move => {
                    let move_event_in = digest_move_message(event_in.data).unwrap();
                    if let Some(&player_entity) = player_lookup.map.get(player_id) {
                        if let Ok(mut move_input) = move_query.get_mut(player_entity) {
                            move_input.x = move_event_in.x;
                            move_input.z = move_event_in.y;
                        }
                    } else {
                        tracing::warn!("Player ID not found: {}", player_id);
                    }
                }
                MessageInType::Jump => {
                    if let Some(&player_entity) = player_lookup.map.get(player_id) {
                        if let Ok(mut move_input) = move_query.get_mut(player_entity) {
                            move_input.y = 1.0;
                        }
                    } else {
                        tracing::warn!("Player ID not found: {}", player_id);
                    }
                }

                MessageInType::Connect => {
                    send_connect_event(player_id, &mut connect_event);
                }
                MessageInType::Invalid => {
                    tracing::error!("Invalied MessageInType");
                }
            }
        }
    });
}

pub fn transport_send_packets(
    mut server: ResMut<MattaServer>,
    mut transport: ResMut<ServerTransport>,
) {
    transport.send_packets(&mut server);
}
