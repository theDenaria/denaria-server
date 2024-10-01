use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};
mod constants;
mod ecs;
mod server;
mod sessions;

use constants::{MAIN_SESSION_ID, TICK_DELTA};
use server::transport::{server::server::ServerConfig, transport::ServerTransport};
use tracing_subscriber::EnvFilter;

fn main() -> io::Result<()> {
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_thread_ids(true)
        .with_thread_names(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Now the tracing macros can be used throughout your application
    tracing::info!("This will dynamically update on the terminal");

    // Setup transport layer
    const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
    let socket: UdpSocket = UdpSocket::bind(SERVER_ADDR)?;
    let server_config = ServerConfig {
        current_time: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        max_clients: 64,
        public_addresses: vec![SERVER_ADDR],
    };

    let mut transport = ServerTransport::new(server_config, socket)?;

    // create default session with player_ids from player1 to player10
    transport.create_session(MAIN_SESSION_ID);

    loop {
        transport.update(TICK_DELTA).unwrap();

        transport.send_packets();

        // make this loop run 60 times per second
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
