use bevy::math::{Quat, Vec3, Vec4};
use bincode;
use serde::{Deserialize, Serialize};

use crate::ecs::systems::setup::LevelObject;

#[derive(Debug)]
pub struct MessageOut {
    // allow dead code because we have some unused message types
    #[allow(dead_code)]
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

    pub fn position_message(positions: Vec<(Vec3, String)>) -> Option<MessageOut> {
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

            let mut serialized = bincode::serialize(&position_event).unwrap();
            serialized.insert(0, 1); // Position Event Type 1
            return Some(MessageOut {
                event_type: MessageOutType::Position,
                data: serialized,
            });
        }
        None
    }

    pub fn rotation_message(rotations: Vec<(Quat, String)>) -> Option<MessageOut> {
        let rotations: Vec<RotationDetails> = rotations
            .iter()
            .map(|(rotation, player_id)| {
                let player_id_bytes = normalize_player_id(player_id.as_str());
                RotationDetails {
                    player_id: player_id_bytes,
                    rotation: Vec4::new(rotation.x, rotation.y, rotation.z, rotation.w),
                }
            })
            .collect();

        if rotations.len() > 0 {
            let rotation_event = RotationMessageOut { rotations };

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

    pub fn spawn_message(player_id: String, position: Vec3, rotation: Quat) -> Option<MessageOut> {
        let spawn_details = SpawnDetails {
            player_id: normalize_player_id(player_id.as_str()),
            position,
            rotation: Vec4::new(rotation.x, rotation.y, rotation.z, rotation.w),
        };

        let spawn_event = SpawnMessageOut {
            spawns: vec![spawn_details],
        };

        let mut serialized = bincode::serialize(&spawn_event).unwrap();

        serialized.insert(0, 0); // Spawn Message Type 0

        Some(MessageOut {
            event_type: MessageOutType::Spawn,
            data: serialized,
        })
    }

    pub fn fire_message(player_id: String, origin: Vec3, direction: Vec3) -> MessageOut {
        let fire_details: FireDetails = FireDetails {
            player_id: normalize_player_id(player_id.as_str()),
            origin,
            direction,
        };

        tracing::info!("{:?}", fire_details);

        let mut serialized = bincode::serialize(&fire_details).unwrap();
        serialized.insert(0, 3); // Fire Message Type 3
        MessageOut {
            event_type: MessageOutType::Fire,
            data: serialized,
        }
    }

    pub fn hit_message(player_id: String, target_id: String, point: Vec3) -> MessageOut {
        let hit_details: HitDetails = HitDetails {
            player_id: normalize_player_id(player_id.as_str()),
            target_id: normalize_player_id(target_id.as_str()),
            point,
        };

        tracing::info!("{:?}", hit_details);

        let mut serialized = bincode::serialize(&hit_details).unwrap();
        serialized.insert(0, 4); // Hit Message Type 4
        MessageOut {
            event_type: MessageOutType::Hit,
            data: serialized,
        }
    }

    pub fn health_message(healths: Vec<(String, f32)>) -> MessageOut {
        let health_details: Vec<HealthDetails> = healths
            .iter()
            .map(|(player_id, health)| {
                let player_id_bytes = normalize_player_id(player_id.as_str());
                HealthDetails {
                    player_id: player_id_bytes,
                    health: *health,
                }
            })
            .collect();

        let mut serialized = bincode::serialize(&health_details).unwrap();
        serialized.insert(0, 6); // Health Message Type 6
        MessageOut {
            event_type: MessageOutType::Health,
            data: serialized,
        }
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
    Spawn = 0,
    Position = 1,
    Rotation = 2,
    Fire = 3,
    Hit = 4,
    Health = 6,
    Disconnect = 10,
}

#[derive(Serialize, Deserialize, Debug)]
struct PositionMessageOut {
    positions: Vec<PositionDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PositionDetails {
    player_id: [u8; 16],
    position: Vec3,
}
#[derive(Serialize, Deserialize, Debug)]
struct RotationMessageOut {
    rotations: Vec<RotationDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RotationDetails {
    player_id: [u8; 16],
    rotation: Vec4,
}

#[derive(Serialize, Deserialize, Debug)]
struct FireDetails {
    player_id: [u8; 16],
    origin: Vec3,
    direction: Vec3,
}

#[derive(Serialize, Deserialize, Debug)]
struct HitDetails {
    player_id: [u8; 16],
    target_id: [u8; 16],
    point: Vec3,
}

#[derive(Serialize, Deserialize, Debug)]
struct HealthDetails {
    player_id: [u8; 16],
    health: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct DisconnectMessage {
    disconnects: Vec<DisconnectDetails>,
}
#[derive(Serialize, Deserialize, Debug)]

struct DisconnectDetails {
    player_id: [u8; 16],
}

#[derive(Serialize, Deserialize, Debug)]
struct SpawnMessageOut {
    spawns: Vec<SpawnDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SpawnDetails {
    player_id: [u8; 16],
    position: Vec3,
    rotation: Vec4,
}
