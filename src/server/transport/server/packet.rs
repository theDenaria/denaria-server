use std::io::{self, Cursor, Write};

use super::{error::TransportServerError, serialize::*};

#[derive(Debug)]
#[repr(u8)]
pub enum PacketType {
    ConnectionRequest = 85,
    Data = 1,
    Disconnect = 2,
    KeepAlive = 3,
}

#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // TODO: Consider boxing types
pub enum Packet<'a> {
    ConnectionRequest {
        connection_prefix: [u8; 3],
        connection_side_id: u8,
        client_identifier: u64,
    },
    KeepAlive {
        client_identifier: u64,
    },
    Data {
        client_identifier: u64,
        payload: &'a [u8],
    },
    Disconnect {
        client_identifier: u64,
    },
}

impl PacketType {
    fn from_u8(value: u8) -> Result<Self, TransportServerError> {
        use PacketType::*;

        let packet_type = match value {
            1 => Data,
            3 => KeepAlive,
            2 => Disconnect,
            85 => ConnectionRequest,
            _ => return Err(TransportServerError::InvalidPacketType),
        };
        Ok(packet_type)
    }

    fn to_u8(self) -> Result<u8, TransportServerError> {
        use PacketType::*;

        let packet_value: u8 = match self {
            Data => 1,
            KeepAlive => 3,
            Disconnect => 2,
            ConnectionRequest => 85,
        };
        Ok(packet_value)
    }
}

impl<'a> Packet<'a> {
    pub fn packet_type(&self) -> PacketType {
        match self {
            Packet::ConnectionRequest { .. } => PacketType::ConnectionRequest,
            Packet::KeepAlive { .. } => PacketType::KeepAlive,
            Packet::Data { .. } => PacketType::Data,
            Packet::Disconnect { .. } => PacketType::Disconnect,
        }
    }

    pub fn id(&self) -> u8 {
        self.packet_type() as u8
    }

    fn write(&self, writer: &mut impl io::Write) -> Result<(), io::Error> {
        match self {
            Packet::ConnectionRequest {
                connection_prefix,
                connection_side_id,
                client_identifier,
            } => {
                writer.write_all(connection_prefix)?;
                writer.write_all(&connection_side_id.to_le_bytes())?;
                writer.write_all(&client_identifier.to_le_bytes())?;
            }
            Packet::KeepAlive { client_identifier } => {
                writer.write_all(&client_identifier.to_le_bytes())?;
            }
            Packet::Data {
                client_identifier,
                payload,
            } => {
                let _ = writer.write_all(&client_identifier.to_le_bytes());
                writer.write_all(payload)?;
            }
            Packet::Disconnect { client_identifier } => {
                let _ = writer.write_all(&client_identifier.to_le_bytes());
            }
        }

        Ok(())
    }

    fn read(packet_type: PacketType, src: &'a [u8]) -> Result<Self, io::Error> {
        let cursor = &mut Cursor::new(src);

        match packet_type {
            PacketType::Data => {
                let client_identifier = read_u64(cursor)?;

                let payload = &src[cursor.position() as usize..];
                Ok(Packet::Data {
                    client_identifier,
                    payload,
                })
            }
            PacketType::ConnectionRequest => {
                let connection_prefix = read_bytes(cursor)?;
                let connection_side_id = read_u8(cursor)?;
                let client_identifier = read_u64(cursor)?;
                Ok(Packet::ConnectionRequest {
                    connection_prefix,
                    connection_side_id,
                    client_identifier,
                })
            }
            PacketType::KeepAlive => {
                let client_identifier = read_u64(cursor)?;

                Ok(Packet::KeepAlive { client_identifier })
            }
            PacketType::Disconnect => {
                let client_identifier = read_u64(cursor)?;
                Ok(Packet::Disconnect { client_identifier })
            }
        }
    }

    pub fn encode(&self, buffer: &mut [u8]) -> Result<usize, TransportServerError> {
        let mut writer = io::Cursor::new(buffer);
        let prefix_byte = self.packet_type().to_u8()?;

        writer.write_all(&prefix_byte.to_le_bytes())?;
        self.write(&mut writer)?;
        Ok(writer.position() as usize)
    }

    pub fn decode(buffer: &'a mut [u8]) -> Result<Self, TransportServerError> {
        let packet_type = buffer[0];
        let packet_type = PacketType::from_u8(packet_type)?;
        let packet = Packet::read(packet_type, &buffer[1..])?;
        Ok(packet)
    }
}
