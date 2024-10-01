use std::{
    collections::HashMap,
    io,
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

use bevy::prelude::Resource;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};

use crate::{
    constants::{MAIN_SESSION_ID, TRANSPORT_MAX_PACKET_BYTES},
    server::server::ClientId,
    sessions::new_session,
};

use super::{
    error::TransportError,
    server::server::{ServerConfig, ServerResult, TransportServer},
};

pub enum ToDenariaServerMessage {
    #[allow(dead_code)]
    ClientConnected {
        client_id: u64,
        addr: SocketAddr,
        payload: Vec<u8>,
        player_id: String,
    },
    ClientDisconnected {
        client_id: u64,
    },
    Payload {
        client_id: u64,
        payload: Vec<u8>,
    },
}

pub enum FromDenariaServerMessage {
    SendPacket {
        client_id: u64,
        packets: Vec<Vec<u8>>,
    },
}

#[derive(Debug, Resource)]
pub struct ServerTransport {
    socket: UdpSocket,
    transport_server: TransportServer,
    buffer: [u8; TRANSPORT_MAX_PACKET_BYTES],
    from_denaria_server_rx: Receiver<FromDenariaServerMessage>,
    from_denaria_server_tx: Sender<FromDenariaServerMessage>,
    player_id_session_map: HashMap<String, u32>,
    session_to_denaria_server_tx: HashMap<u32, Sender<ToDenariaServerMessage>>,
    client_id_to_server_tx_map: HashMap<u64, Sender<ToDenariaServerMessage>>,
}

impl ServerTransport {
    pub fn new(server_config: ServerConfig, socket: UdpSocket) -> Result<Self, std::io::Error> {
        socket.set_nonblocking(true)?;

        let transport_server = TransportServer::new(server_config);

        let (from_denaria_server_tx, from_denaria_server_rx) =
            unbounded::<FromDenariaServerMessage>();

        Ok(Self {
            socket,
            transport_server,
            buffer: [0; TRANSPORT_MAX_PACKET_BYTES],
            from_denaria_server_rx,
            from_denaria_server_tx,
            player_id_session_map: HashMap::new(),
            session_to_denaria_server_tx: HashMap::new(),
            client_id_to_server_tx_map: HashMap::new(),
        })
    }

    pub fn create_session(&mut self, id: u32) {
        // create bevy app in a new thread giving the channel receiver to the DenariaServer
        let (tx, rx) = unbounded::<ToDenariaServerMessage>();

        let from_denaria_server_tx = self.from_denaria_server_tx.clone();

        self.session_to_denaria_server_tx.insert(id, tx);

        std::thread::spawn(move || {
            new_session(from_denaria_server_tx, rx);
        });
    }

    /// Returns the server public address
    pub fn addresses(&self) -> Vec<SocketAddr> {
        self.transport_server.addresses()
    }

    /// Returns the maximum number of clients that can be connected.
    pub fn max_clients(&self) -> usize {
        self.transport_server.max_clients()
    }

    /// Returns current number of clients connected.
    pub fn connected_clients(&self) -> usize {
        self.transport_server.connected_clients()
    }

    /// Returns the client address if connected.
    pub fn client_addr(&self, client_id: ClientId) -> Option<SocketAddr> {
        self.transport_server.client_addr(client_id.raw())
    }

    /// Disconnects all connected clients.
    /// This sends the disconnect packet instantly, use this when closing/exiting games,
    pub fn disconnect_all(&mut self) {
        for client_id in self.transport_server.clients_id() {
            let server_result = self.transport_server.disconnect(client_id);
            // get tx map by client id and send disconnect message
            if let Some(sender) = self.client_id_to_server_tx_map.get_mut(&client_id) {
                if let Err(e) =
                    sender.send(ToDenariaServerMessage::ClientDisconnected { client_id })
                {
                    tracing::error!("Failed to send disconnect message to client {client_id}: {e}");
                }
            }
            handle_server_result(
                server_result,
                &self.socket,
                &mut self.player_id_session_map,
                &self.session_to_denaria_server_tx,
                &mut self.client_id_to_server_tx_map,
            );
        }
    }

    /// Returns the duration since the connected client last received a packet.
    /// Usefull to detect users that are timing out.
    pub fn time_since_last_received_packet(&self, client_id: ClientId) -> Option<Duration> {
        self.transport_server
            .time_since_last_received_packet(client_id.raw())
    }

    /// Advances the transport by the duration, and receive packets from the network.
    pub fn update(&mut self, duration: Duration) -> Result<(), TransportError> {
        self.transport_server.update(duration);

        loop {
            match self.socket.recv_from(&mut self.buffer) {
                Ok((len, addr)) => {
                    let server_result = self
                        .transport_server
                        .process_packet(addr, &mut self.buffer[..len]);

                    if let Some(new_session_details) = handle_server_result(
                        server_result,
                        &self.socket,
                        &mut self.player_id_session_map,
                        &self.session_to_denaria_server_tx,
                        &mut self.client_id_to_server_tx_map,
                    ) {
                        self.create_session(new_session_details.id);
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => break,
                Err(ref e) if e.kind() == io::ErrorKind::ConnectionReset => continue,
                Err(e) => return Err(e.into()),
            };
        }

        for client_id in self.transport_server.clients_id() {
            let server_result = self.transport_server.update_client(client_id);
            handle_server_result(
                server_result,
                &self.socket,
                &mut self.player_id_session_map,
                &self.session_to_denaria_server_tx,
                &mut self.client_id_to_server_tx_map,
            );
        }
        // for disconnection_id in server.disconnections_id() {
        //     let server_result = self.transport_server.disconnect(disconnection_id.raw());
        //     handle_server_result(server_result, &self.socket);
        // }

        Ok(())
    }

    /// Send packets to connected clients.
    pub fn send_packets(&mut self) {
        self.handle_messages();
    }

    fn handle_messages(&mut self) {
        let start_time = Instant::now();
        loop {
            if start_time.elapsed() >= Duration::from_millis(10) {
                break; // Time limit reached
            }
            match self.from_denaria_server_rx.try_recv() {
                Ok(message) => self.send_message(message),
                Err(TryRecvError::Empty) => break, // No more messages to process
                Err(TryRecvError::Disconnected) => {
                    tracing::error!("Channel to DenariaServer disconnected");
                    break;
                }
            }
        }
    }

    fn send_message(&mut self, message: FromDenariaServerMessage) {
        match message {
            FromDenariaServerMessage::SendPacket { client_id, packets } => {
                for packet in packets {
                    match self
                        .transport_server
                        .generate_payload_packet(client_id, &packet)
                    {
                        Ok((addr, payload)) => {
                            if let Err(e) = self.socket.send_to(payload, addr) {
                                tracing::error!(
                                    "Failed to send packet to client {client_id} ({addr}): {e}"
                                );
                            }
                            break;
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to encrypt payload packet for client {client_id}: {e}"
                            );
                            break;
                        }
                    }
                }
            }
        }
    }
}

struct NewSessionDetails {
    id: u32,
    player_ids: Vec<String>,
}

fn handle_server_result(
    server_result: ServerResult,
    socket: &UdpSocket,
    player_id_session_map: &mut HashMap<String, u32>,
    session_to_denaria_server_tx: &HashMap<u32, Sender<ToDenariaServerMessage>>,
    client_id_to_server_tx_map: &mut HashMap<u64, Sender<ToDenariaServerMessage>>,
) -> Option<NewSessionDetails> {
    let send_packet = |packet: &[u8], addr: SocketAddr| {
        if let Err(err) = socket.send_to(packet, addr) {
            tracing::error!("Failed to send packet to {addr}: {err}");
        }
    };

    match server_result {
        ServerResult::None => {}
        ServerResult::PacketToSend { payload, addr } => {
            send_packet(payload, addr);
        }
        ServerResult::Payload { client_id, payload } => {
            match client_id_to_server_tx_map.get(&client_id) {
                Some(sender) => {
                    if let Err(e) = sender.send(ToDenariaServerMessage::Payload {
                        client_id,
                        payload: payload.to_vec(),
                    }) {
                        tracing::error!("Failed to send payload to client {client_id}: {e}");
                    }
                }
                None => {
                    tracing::error!("Server (in a session) not found for client {client_id}");
                }
            }
        }
        ServerResult::ClientConnected {
            client_id,
            addr,
            payload,
            player_id,
        } => {
            if let Some(sender) = session_to_denaria_server_tx.get(&MAIN_SESSION_ID) {
                if let Err(e) = sender.send(ToDenariaServerMessage::ClientConnected {
                    client_id,
                    addr,
                    payload: payload.to_vec(),
                    player_id: player_id.clone(),
                }) {
                    tracing::error!(
                        "Failed to send client connected message to client {client_id}: {e}"
                    );
                }
                player_id_session_map.insert(player_id, MAIN_SESSION_ID);
                client_id_to_server_tx_map.insert(client_id, sender.clone());
            }
            send_packet(payload, addr);
        }
        ServerResult::ClientDisconnected {
            client_id,
            addr,
            payload,
        } => {
            if let Some(sender) = client_id_to_server_tx_map.get(&client_id) {
                if let Err(e) =
                    sender.send(ToDenariaServerMessage::ClientDisconnected { client_id })
                {
                    tracing::error!(
                        "Failed to send client disconnected message to client {client_id}: {e}"
                    );
                }
            }
            if let Some(payload) = payload {
                send_packet(payload, addr);
            }
        }
    }
    None
}
