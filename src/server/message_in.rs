use crate::ecs::events::{JumpEvent, LookEvent, MoveEvent, SpawnEvent};
use crate::server::packet::SerializationError;
use bevy::math::Vec4;
use bevy::prelude::Entity;
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Debug)]
pub struct MessageIn {
    pub event_type: MessageInType,
    pub data: Vec<u8>,
    pub player_id: String,
}

impl MessageIn {
    pub fn new(bytes: Vec<u8>, player_id: String) -> Result<MessageIn, &'static str> {
        if bytes.len() < 1 {
            return Err("Not enough bytes for EventIn");
        }

        let event_type = MessageInType::try_from(bytes[0]).map_err(|_| "Invalid event type")?;
        let data = &bytes[1..];

        Ok(MessageIn {
            event_type,
            data: data.to_vec(),
            player_id: player_id.clone(),
        })
    }

    pub fn to_move_event(&self, player_entity: Entity) -> Result<MoveEvent, SerializationError> {
        // let data_slice: &[u8] = &self.data;
        if self.data.len() < 8 {
            println!("Insufficent bytes: {:?}", self.data);
            return Err(SerializationError::BufferTooShort);
        }
        let mut reader = Cursor::new(&self.data);

        let x = reader.read_f32::<LittleEndian>()?;
        let y = reader.read_f32::<LittleEndian>()?;

        Ok(MoveEvent {
            entity: player_entity,
            x,
            y,
        })
    }
    pub fn to_look_event(&self, player_entity: Entity) -> Result<LookEvent, SerializationError> {
        if self.data.len() < 12 {
            println!("Insufficent bytes: {:?}", self.data);
            return Err(SerializationError::BufferTooShort);
        }
        let mut reader = Cursor::new(&self.data);

        let x = reader.read_f32::<LittleEndian>()?;
        let y = reader.read_f32::<LittleEndian>()?;
        let z = reader.read_f32::<LittleEndian>()?;
        let w = reader.read_f32::<LittleEndian>()?;

        Ok(LookEvent {
            entity: player_entity,
            direction: Vec4::new(x, y, z, w),
        })
    }

    pub fn to_jump_event(&self, player_entity: Entity) -> Result<JumpEvent, SerializationError> {
        Ok(JumpEvent {
            entity: player_entity,
        })
    }

    pub fn to_spawn_event(&self) -> Result<SpawnEvent, SerializationError> {
        Ok(SpawnEvent {
            player_id: self.player_id.clone(),
        })
    }
}

#[derive(Debug)]
pub enum MessageInType {
    Spawn = 0,
    Move = 2,
    Rotation = 3,
    Jump = 4,
    Invalid = 99,
    // SessionCreate = 100,
    // SessionJoin = 101,
}

impl TryFrom<u8> for MessageInType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MessageInType::Spawn),
            2 => Ok(MessageInType::Move),
            3 => Ok(MessageInType::Rotation),
            4 => Ok(MessageInType::Jump),
            // 100 => Ok(MessageInType::SessionCreate),
            _ => Ok(MessageInType::Invalid),
        }
    }
}
