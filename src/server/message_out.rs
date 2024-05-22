use bincode;
use rapier3d::na::Vector3;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct MessageOut {
    pub event_type: MessageOutType,
    pub data: Vec<u8>,
}

impl MessageOut {
    pub fn get_with_event_header(&self, identifier: Vec<u8>) -> Vec<u8> {
        let mut with_header: Vec<u8> = vec![];
        with_header.push(1);

        with_header.extend(identifier);
        with_header.push(0);
        with_header.extend(self.data.clone());
        with_header
    }

    pub fn position_message(positions: Vec<(Vector3<f32>, String)>) -> Option<MessageOut> {
        let position_details: Vec<PositionDetails> = positions
            .iter()
            .map(|(position, player_id)| {
                let player_id_bytes = normalize_player_id(player_id.as_str());
                PositionDetails {
                    player_id: player_id_bytes,
                    position: *position,
                }
            })
            .collect();

        if positions.len() > 0 {
            let position_event = PositionMessageOut {
                positions: position_details,
            };
            tracing::info!("{:?}", position_event);

            let mut serialized = bincode::serialize(&position_event).unwrap();
            serialized.insert(0, 1); // Position Event Type 1
            return Some(MessageOut {
                event_type: MessageOutType::Position,
                data: serialized,
            });
        }
        None
    }

    pub fn rotation_message(rotations: Vec<(Vector3<f32>, String)>) -> Option<MessageOut> {
        let rotations: Vec<RotationDetails> = rotations
            .iter()
            .map(|(rotation, player_id)| {
                let player_id_bytes = normalize_player_id(player_id.as_str());
                RotationDetails {
                    player_id: player_id_bytes,
                    rotation: *rotation,
                }
            })
            .collect();

        if rotations.len() > 0 {
            let rotation_event = RotationMessageOut { rotations };
            tracing::info!("{:?}", rotation_event);

            let mut serialized = bincode::serialize(&rotation_event).unwrap();
            serialized.insert(0, 2); // Rotation Event Type 1
            return Some(MessageOut {
                event_type: MessageOutType::Rotation,
                data: serialized,
            });
        }
        None
    }

    pub fn disconnect_message(player_ids: Vec<&String>) -> Option<MessageOut> {
        let player_num = player_ids.len() as u32;
        if player_num < 1 {
            return None;
        }
        let mut disconnects: Vec<DisconnectDetails> = vec![];

        for player_id in player_ids {
            let player_id_bytes = normalize_player_id(player_id.as_str());
            disconnects.push(DisconnectDetails {
                player_id: player_id_bytes,
            });
        }

        let disconnect_event = DisconnectMessage { disconnects };

        let mut serialized = bincode::serialize(&disconnect_event).unwrap();

        serialized.insert(0, 10); // Disconnect Event Type 10

        Some(MessageOut {
            event_type: MessageOutType::Disconnect,
            data: serialized,
        })
    }
}

fn normalize_player_id(player_id: &str) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    let player_id_bytes = player_id.as_bytes();
    let len = player_id_bytes.len().min(16);
    bytes[..len].copy_from_slice(&player_id_bytes[..len]);
    bytes
}

#[derive(Debug)]
pub enum MessageOutType {
    Position = 1,
    Rotation = 2,
    Disconnect = 10,
}

#[derive(Serialize, Deserialize, Debug)]
struct PositionMessageOut {
    positions: Vec<PositionDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PositionDetails {
    player_id: [u8; 16],
    position: Vector3<f32>,
}
#[derive(Serialize, Deserialize, Debug)]
struct RotationMessageOut {
    rotations: Vec<RotationDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RotationDetails {
    player_id: [u8; 16],
    rotation: Vector3<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DisconnectMessage {
    disconnects: Vec<DisconnectDetails>,
}
#[derive(Serialize, Deserialize, Debug)]

struct DisconnectDetails {
    player_id: [u8; 16],
}
