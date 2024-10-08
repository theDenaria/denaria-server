use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    constants::{
        TRANSPORT_MAX_CLIENTS, TRANSPORT_MAX_PACKET_BYTES, TRANSPORT_MAX_PENDING_CLIENTS,
        TRANSPORT_SEND_RATE,
    },
    server::transport::server::packet::Packet,
};

use super::error::TransportServerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    PendingResponse,
    Authenticating,
    Connected,
}

#[derive(Debug, Clone)]
struct Connection {
    confirmed: bool,
    client_id: u64,
    state: ConnectionState,
    is_authenticated: Arc<Mutex<(bool, String)>>,
    // TODO MAYBE user_data: [u8; NETCODE_USER_DATA_BYTES],
    addr: SocketAddr,
    last_packet_received_time: Duration,
    last_packet_send_time: Duration,
    timeout_seconds: i32,
    expire_timestamp: u64,
}

/// A server that can generate packets from connect clients, that are encrypted, or process
/// incoming encrypted packets from clients. The server is agnostic from the transport layer, only
/// consuming and generating bytes that can be transported in any way desired.
#[derive(Debug)]
pub struct TransportServer {
    clients: Box<[Option<Connection>]>,
    pending_clients: HashMap<SocketAddr, Connection>,
    max_clients: usize,
    public_addresses: Vec<SocketAddr>,
    current_time: Duration,
    out: [u8; TRANSPORT_MAX_PACKET_BYTES],
}

/// Result from processing an packet in the server
#[derive(Debug, PartialEq, Eq)]
pub enum ServerResult<'a, 's> {
    /// Nothing needs to be done.
    None,
    /// A packet to be sent back to the processed address.
    PacketToSend {
        addr: SocketAddr,
        payload: &'s mut [u8],
    },
    /// A payload received from the client.
    Payload {
        client_id: u64,
        payload: &'a [u8],
    },
    /// A new client has connected
    ClientConnected {
        client_id: u64,
        addr: SocketAddr,
        payload: &'s mut [u8],
        player_id: String,
    },
    /// The client connection has been terminated.
    ClientDisconnected {
        client_id: u64,
        addr: SocketAddr,
        payload: Option<&'s mut [u8]>,
    },
    CreateSession {
        id: u32,
        player_ids: Vec<String>,
    },
}

pub struct ServerConfig {
    pub current_time: Duration,
    /// Maximum numbers of clients that can be connected at a time
    pub max_clients: usize,
    /// Publicly available addresses to which clients will attempt to connect.
    pub public_addresses: Vec<SocketAddr>,
}

impl TransportServer {
    pub fn new(config: ServerConfig) -> Self {
        if config.max_clients > TRANSPORT_MAX_CLIENTS {
            // TODO: do we really need to set a max?
            //       only using for token entries
            panic!("The max clients allowed is {}", TRANSPORT_MAX_CLIENTS);
        }

        let clients = vec![None; config.max_clients].into_boxed_slice();

        Self {
            clients,
            pending_clients: HashMap::new(),
            max_clients: config.max_clients,

            public_addresses: config.public_addresses,
            current_time: config.current_time,
            out: [0u8; TRANSPORT_MAX_PACKET_BYTES],
        }
    }

    pub fn addresses(&self) -> Vec<SocketAddr> {
        self.public_addresses.clone()
    }

    pub fn current_time(&self) -> Duration {
        self.current_time
    }

    // /// Returns the user data from the connected client.
    // pub fn user_data(&self, client_id: u64) -> Option<[u8; NETCODE_USER_DATA_BYTES]> {
    //     if let Some(client) = find_client_by_id(&self.clients, client_id) {
    //         return Some(client.user_data);
    //     }

    //     None
    // }

    /// Returns the duration since the connected client last received a packet.
    /// Usefull to detect users that are timing out.
    pub fn time_since_last_received_packet(&self, client_id: u64) -> Option<Duration> {
        if let Some(client) = find_client_by_id(&self.clients, client_id) {
            let time = self.current_time - client.last_packet_received_time;
            return Some(time);
        }

        None
    }

    /// Returns the client address if connected.
    pub fn client_addr(&self, client_id: u64) -> Option<SocketAddr> {
        if let Some(client) = find_client_by_id(&self.clients, client_id) {
            return Some(client.addr);
        }

        None
    }

    fn handle_connection_request<'a>(
        &mut self,
        addr: SocketAddr,
        connection_prefix: [u8; 3],
        client_identifier: u64,
    ) -> Result<ServerResult<'a, '_>, TransportServerError> {
        let addr_already_connected = find_client_mut_by_addr(&mut self.clients, addr).is_some();
        let id_already_connected =
            find_client_mut_by_id(&mut self.clients, client_identifier).is_some();

        if id_already_connected || addr_already_connected {
            tracing::debug!(
                "Connection request denied: client {} already connected (address: {}).",
                client_identifier,
                addr
            );
            return Ok(ServerResult::None);
        }

        if !self.pending_clients.contains_key(&addr)
            && self.pending_clients.len() >= TRANSPORT_MAX_PENDING_CLIENTS
        {
            tracing::warn!(
                "Connection request denied: reached max amount allowed of pending clients ({}).",
                TRANSPORT_MAX_PENDING_CLIENTS
            );
            return Ok(ServerResult::None);
        }

        if self.clients.iter().flatten().count() >= self.max_clients {
            self.pending_clients.remove(&addr);
            // TODO: Maybe implement ConnectionDenied message
            return Ok(ServerResult::None);
        }

        let packet = Packet::ConnectionRequest {
            connection_prefix,
            connection_side_id: 2,
            client_identifier,
        };

        let len = packet.encode(&mut self.out)?;

        tracing::trace!("Connection request from Client {}", client_identifier);

        let pending = self
            .pending_clients
            .entry(addr)
            .or_insert_with(|| Connection {
                confirmed: false,
                is_authenticated: Arc::new(Mutex::new((false, String::new()))),
                client_id: client_identifier,
                last_packet_received_time: self.current_time,
                last_packet_send_time: self.current_time,
                addr,
                state: ConnectionState::PendingResponse,
                timeout_seconds: 10,
                // write code to calculate based on timeout_seconds and current_time
                expire_timestamp: self.current_time.as_secs() + 10 as u64,
            });
        pending.last_packet_received_time = self.current_time;
        pending.last_packet_send_time = self.current_time;

        Ok(ServerResult::PacketToSend {
            addr,
            payload: &mut self.out[..len],
        })
    }

    /// Returns an encoded packet payload to be sent to the client
    pub fn generate_payload_packet<'s>(
        &'s mut self,
        client_identifier: u64,
        payload: &[u8],
    ) -> Result<(SocketAddr, &'s mut [u8]), TransportServerError> {
        if let Some(client) = find_client_mut_by_id(&mut self.clients, client_identifier) {
            let packet = Packet::Data {
                client_identifier,
                payload,
            };
            let len = packet.encode(&mut self.out)?;

            client.last_packet_send_time = self.current_time;

            return Ok((client.addr, &mut self.out[..len]));
        }

        Err(TransportServerError::ClientNotFound)
    }

    /// Process an packet from the especifed address. Returns a server result, check out
    /// [ServerResult].
    pub fn process_packet<'a, 's>(
        &'s mut self,
        addr: SocketAddr,
        buffer: &'a mut [u8],
    ) -> ServerResult<'a, 's> {
        match self.process_packet_internal(addr, buffer) {
            Err(e) => {
                tracing::error!("Failed to process packet: {}", e);
                ServerResult::None
            }
            Ok(r) => r,
        }
    }

    fn process_packet_internal<'a, 's>(
        &'s mut self,
        addr: SocketAddr,
        buffer: &'a mut [u8],
    ) -> Result<ServerResult<'a, 's>, TransportServerError> {
        // Handle connected client
        if let Some((slot, client)) = find_client_mut_by_addr(&mut self.clients, addr) {
            let packet = Packet::decode(buffer)?;

            client.last_packet_received_time = self.current_time;
            match client.state {
                ConnectionState::Connected => match packet {
                    Packet::Disconnect {
                        client_identifier: _,
                    } => {
                        client.state = ConnectionState::Disconnected;
                        let client_id = client.client_id;
                        self.clients[slot] = None;
                        tracing::trace!("Client {} requested to disconnect", client_id);
                        return Ok(ServerResult::ClientDisconnected {
                            client_id,
                            addr,
                            payload: None,
                        });
                    }
                    Packet::Data {
                        client_identifier: _,
                        payload,
                    } => {
                        if !client.confirmed {
                            tracing::trace!("Confirmed connection for Client {}", client.client_id);
                            client.confirmed = true;
                        }
                        return Ok(ServerResult::Payload {
                            client_id: client.client_id,
                            payload,
                        });
                    }
                    Packet::KeepAlive { .. } => {
                        if !client.confirmed {
                            tracing::trace!("Confirmed connection for Client {}", client.client_id);
                            client.confirmed = true;
                        }
                        return Ok(ServerResult::None);
                    }
                    _ => return Ok(ServerResult::None),
                },
                _ => return Ok(ServerResult::None),
            }
        }

        // Handle pending client
        if let Some(pending) = self.pending_clients.get_mut(&addr) {
            let packet = Packet::decode(buffer)?;

            pending.last_packet_received_time = self.current_time;
            tracing::trace!(
                "Received packet from pending client ({}): {:?}",
                addr,
                packet.packet_type()
            );
            match packet {
                Packet::ConnectionRequest {
                    connection_prefix,
                    connection_side_id,
                    client_identifier,
                } => {
                    if (connection_side_id) == 1 {
                        return self.handle_connection_request(
                            addr,
                            connection_prefix,
                            client_identifier,
                        );
                    } else {
                        return Ok(ServerResult::None);
                    }
                }
                // If its Data from pending client it has to be the application level connection request in the payload
                Packet::Data {
                    payload,
                    client_identifier,
                } => {
                    let mut pending = self.pending_clients.remove(&addr).unwrap();

                    match pending.state {
                        ConnectionState::Authenticating => {
                            let is_authenticated = pending.is_authenticated.lock().unwrap();
                            if is_authenticated.0 {
                                if find_client_slot_by_id(&self.clients, client_identifier)
                                    .is_some()
                                {
                                    tracing::debug!(
                                        "Ignored connection response for Client {}, already connected.",
                                        client_identifier
                                    );
                                    return Ok(ServerResult::None);
                                }

                                match self.clients.iter().position(|c| c.is_none()) {
                                    None => {
                                        let packet = Packet::Disconnect { client_identifier };
                                        let len = packet.encode(&mut self.out)?;
                                        pending.state = ConnectionState::Disconnected;

                                        pending.last_packet_send_time = self.current_time;
                                        return Ok(ServerResult::PacketToSend {
                                            addr,
                                            payload: &mut self.out[..len],
                                        });
                                    }
                                    Some(client_index) => {
                                        pending.state = ConnectionState::Connected;
                                        pending.last_packet_send_time = self.current_time;

                                        let packet = Packet::KeepAlive { client_identifier };
                                        let len = packet.encode(&mut self.out)?;

                                        let client_id: u64 = pending.client_id;

                                        self.clients[client_index] = Some(pending.clone());

                                        let player_id = is_authenticated.1.clone();

                                        return Ok(ServerResult::ClientConnected {
                                            client_id,
                                            addr,
                                            player_id,
                                            payload: &mut self.out[..len],
                                        });
                                    }
                                }
                            } else {
                                self.pending_clients.insert(addr, pending.clone());
                            }
                            return Ok(ServerResult::None);
                        }
                        ConnectionState::PendingResponse => {
                            pending.state = ConnectionState::Authenticating;

                            let bytes = payload.to_vec();
                            let channel_id = bytes[0];
                            let messages_len = bytes[1];
                            let message_type = bytes[5];

                            if bytes.len() < 17
                                || channel_id != 0
                                || messages_len != 1
                                || message_type != 0
                            {
                                return Err(TransportServerError::InvalidPacketType);
                            }

                            let (player_id_bytes, session_ticket_bytes) = bytes[6..].split_at(16);

                            let player_id = String::from_utf8(player_id_bytes.to_vec())
                                .map_err(|_| TransportServerError::InvalidPlayerId)?
                                .trim_end_matches(char::from(0))
                                .to_string();

                            tracing::trace!("Authenticating: {:?}", player_id);

                            let session_ticket = String::from_utf8(session_ticket_bytes.to_vec())
                                .map_err(|_| TransportServerError::InvalidSessionTicket)?
                                .trim_end_matches(char::from(0))
                                .to_string();

                            let is_authenticated = pending.is_authenticated.clone();

                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                rt.block_on(async move {
                                    authenticate_player(
                                        player_id,
                                        session_ticket,
                                        is_authenticated,
                                    )
                                    .await;
                                });
                            });

                            pending.last_packet_send_time = self.current_time;
                            let packet = Packet::KeepAlive { client_identifier };
                            let len = packet.encode(&mut self.out)?;

                            self.pending_clients.insert(addr, pending);
                            return Ok(ServerResult::PacketToSend {
                                addr,
                                payload: &mut self.out[..len],
                            });
                        }
                        _ => return Ok(ServerResult::None),
                    }
                }
                _ => return Ok(ServerResult::None),
            }
        } else {
            // Handle new client
            let packet = Packet::decode(buffer)?;
            match packet {
                Packet::ConnectionRequest {
                    connection_prefix,
                    connection_side_id,
                    client_identifier,
                } => {
                    if (connection_side_id) == 1 {
                        return self.handle_connection_request(
                            addr,
                            connection_prefix,
                            client_identifier,
                        );
                    } else {
                        return Ok(ServerResult::None);
                    }
                }
                Packet::CreateSession {
                    client_identifier: _,
                    session_id,
                    player_ids,
                } => {
                    return Ok(ServerResult::CreateSession {
                        id: session_id,
                        player_ids,
                    });
                }
                _ => Ok(ServerResult::None),
            }
        }
    }

    pub fn clients_slot(&self) -> Vec<usize> {
        self.clients
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| if slot.is_some() { Some(index) } else { None })
            .collect()
    }

    /// Returns the ids from the connected clients (iterator).
    pub fn clients_id_iter(&self) -> impl Iterator<Item = u64> + '_ {
        self.clients
            .iter()
            .filter_map(|slot| slot.as_ref().map(|client| client.client_id))
    }

    /// Returns the ids from the connected clients.
    pub fn clients_id(&self) -> Vec<u64> {
        self.clients_id_iter().collect()
    }

    /// Returns the maximum number of clients that can be connected.
    pub fn max_clients(&self) -> usize {
        self.max_clients
    }

    /// Update the maximum numbers of clients that can be connected
    ///
    /// Changing the `max_clients` to a lower value than the current number of connect clients
    /// does not disconnect clients. So [`NetcodeServer::connected_clients()`] can return a higher value than [`NetcodeServer::max_clients()`].
    pub fn set_max_clients(&mut self, max_clients: usize) {
        self.max_clients = max_clients;
    }

    /// Returns current number of clients connected.
    pub fn connected_clients(&self) -> usize {
        self.clients.iter().filter(|slot| slot.is_some()).count()
    }

    /// Advance the server current time, and remove any pending connections that have expired.
    pub fn update(&mut self, duration: Duration) {
        self.current_time += duration;

        for client in self.pending_clients.values_mut() {
            if self.current_time.as_secs() > client.expire_timestamp {
                tracing::debug!(
                    "Pending Client {} disconnected, connection token expired.",
                    client.client_id
                );
                client.state = ConnectionState::Disconnected;
            }
        }

        self.pending_clients
            .retain(|_, c| c.state != ConnectionState::Disconnected);
    }

    pub fn update_client(&mut self, client_id: u64) -> ServerResult<'_, '_> {
        let slot = match find_client_slot_by_id(&self.clients, client_id) {
            None => return ServerResult::None,
            Some(slot) => slot,
        };

        if let Some(client) = &mut self.clients[slot] {
            let connection_timed_out = client.timeout_seconds > 0
                && (client.last_packet_received_time
                    + Duration::from_secs(client.timeout_seconds as u64)
                    < self.current_time);
            if connection_timed_out {
                tracing::debug!(
                    "Client {} disconnected, connection timed out",
                    client.client_id
                );
                client.state = ConnectionState::Disconnected;
            }

            if client.state == ConnectionState::Disconnected {
                let packet = Packet::Disconnect {
                    client_identifier: client_id,
                };

                let addr = client.addr;
                self.clients[slot] = None;

                let len = match packet.encode(&mut self.out) {
                    Err(e) => {
                        tracing::error!("Failed to encode disconnect packet: {}", e);
                        return ServerResult::ClientDisconnected {
                            client_id,
                            addr,
                            payload: None,
                        };
                    }
                    Ok(len) => len,
                };

                return ServerResult::ClientDisconnected {
                    client_id,
                    addr,
                    payload: Some(&mut self.out[..len]),
                };
            }

            if client.last_packet_send_time + TRANSPORT_SEND_RATE <= self.current_time {
                let packet = Packet::KeepAlive {
                    client_identifier: client_id as u64,
                };

                let len = match packet.encode(&mut self.out) {
                    Err(e) => {
                        tracing::error!("Failed to encode keep alive packet: {}", e);
                        return ServerResult::None;
                    }
                    Ok(len) => len,
                };

                client.last_packet_send_time = self.current_time;
                return ServerResult::PacketToSend {
                    addr: client.addr,
                    payload: &mut self.out[..len],
                };
            }
        }

        ServerResult::None
    }

    pub fn is_client_connected(&self, client_id: u64) -> bool {
        find_client_slot_by_id(&self.clients, client_id).is_some()
    }

    /// Disconnect an client and returns its address and a disconnect packet to be sent to them.
    // TODO: we can return Result<PacketToSend, NetcodeError>
    //       but the library user would need to be aware that he has to run
    //       the same code as Result::ClientDisconnected
    pub fn disconnect(&mut self, client_id: u64) -> ServerResult<'_, '_> {
        if let Some(slot) = find_client_slot_by_id(&self.clients, client_id) {
            let client = self.clients[slot].take().unwrap();
            let packet = Packet::Disconnect {
                client_identifier: client_id,
            };

            let len = match packet.encode(&mut self.out) {
                Err(e) => {
                    tracing::error!("Failed to encode disconnect packet: {}", e);
                    return ServerResult::ClientDisconnected {
                        client_id,
                        addr: client.addr,
                        payload: None,
                    };
                }
                Ok(len) => len,
            };
            return ServerResult::ClientDisconnected {
                client_id,
                addr: client.addr,
                payload: Some(&mut self.out[..len]),
            };
        }

        ServerResult::None
    }
}

fn find_client_mut_by_id(
    clients: &mut [Option<Connection>],
    client_id: u64,
) -> Option<&mut Connection> {
    clients
        .iter_mut()
        .flatten()
        .find(|c| c.client_id == client_id)
}

fn find_client_by_id(clients: &[Option<Connection>], client_id: u64) -> Option<&Connection> {
    clients.iter().flatten().find(|c| c.client_id == client_id)
}

fn find_client_slot_by_id(clients: &[Option<Connection>], client_id: u64) -> Option<usize> {
    clients.iter().enumerate().find_map(|(i, c)| match c {
        Some(c) if c.client_id == client_id => Some(i),
        _ => None,
    })
}

fn find_client_mut_by_addr(
    clients: &mut [Option<Connection>],
    addr: SocketAddr,
) -> Option<(usize, &mut Connection)> {
    clients.iter_mut().enumerate().find_map(|(i, c)| match c {
        Some(c) if c.addr == addr => Some((i, c)),
        _ => None,
    })
}

async fn authenticate_player(
    player_id: String,
    session_ticket: String,
    is_authenticated: Arc<Mutex<(bool, String)>>,
) {
    let client = reqwest::Client::new();
    let playfab_api_key = std::env::var("PLAYFAB_API_KEY").unwrap();
    let playfab_api_url = std::env::var("PLAYFAB_API_URL").unwrap();
    let response = client
        .post(format!(
            "{}/Server/AuthenticateSessionTicket", // /Server/AuthenticateSessionTicket
            playfab_api_url
        ))
        .header("X-SecretKey", playfab_api_key)
        .json(&serde_json::json!({
            "SessionTicket": session_ticket,
        }))
        .send()
        .await;
    match response {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(response_body) => {
                        let mut is_authenticated = is_authenticated.lock().unwrap();
                        is_authenticated.0 = true;
                        is_authenticated.1 = player_id;
                    }
                    Err(e) => {
                        tracing::error!("Failed to authenticate player: {}", e);
                    }
                }
            } else {
                tracing::error!("Failed to authenticate player: {}", response.status());
            }
        }
        Err(e) => {
            tracing::error!("Failed to authenticate player: {}", e);
        }
    }
}
