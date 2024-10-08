use std::{error, fmt, io};

use crate::{constants::TRANSPORT_MAX_PAYLOAD_BYTES, server::error::DisconnectReason};

// allow dead code because we have some unused message types
#[allow(dead_code)]
#[derive(Debug)]
pub enum TransportServerError {
    /// The type of the packet is invalid.
    InvalidPacketType,
    /// Invalid player id in connect packet
    InvalidPlayerId,
    /// Invalid session ticket in connect packet
    InvalidSessionTicket,
    /// Packet size is too small to be a netcode packet.
    PacketTooSmall,
    /// Payload is above the maximum limit
    PayloadAboveLimit,
    /// The processed packet is duplicated
    DuplicatedSequence,
    /// No more host are available in the connect token..
    NoMoreServers,
    /// The connect token has expired.
    Expired,
    /// The client is disconnected.
    Disconnected(DisconnectReason),
    /// The server address is not in the connect token.
    NotInHostList,
    /// Client was not found.
    ClientNotFound,
    /// Client is not connected.
    ClientNotConnected,
    /// IO error.
    IoError(io::Error),
}

impl fmt::Display for TransportServerError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use TransportServerError::*;

        match *self {
            InvalidPacketType => write!(fmt, "invalid packet type"),
            InvalidPlayerId => write!(fmt, "invalid player_id bytes to deserialize"),
            InvalidSessionTicket => write!(fmt, "invalid session ticket bytes to deserialize"),
            PacketTooSmall => write!(fmt, "packet is too small"),
            PayloadAboveLimit => write!(
                fmt,
                "payload is above the {} bytes limit",
                TRANSPORT_MAX_PAYLOAD_BYTES
            ),
            Expired => write!(fmt, "connection expired"),
            DuplicatedSequence => write!(fmt, "sequence already received"),
            Disconnected(reason) => write!(fmt, "disconnected: {}", reason),
            NoMoreServers => write!(fmt, "client has no more servers to connect"),
            NotInHostList => write!(fmt, "token does not contain the server address"),
            ClientNotFound => write!(fmt, "client was not found"),
            ClientNotConnected => write!(fmt, "client is disconnected or connecting"),
            IoError(ref err) => write!(fmt, "{}", err),
        }
    }
}

impl error::Error for TransportServerError {}

impl From<io::Error> for TransportServerError {
    fn from(inner: io::Error) -> Self {
        TransportServerError::IoError(inner)
    }
}
