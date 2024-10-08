use std::time::Duration;

pub const TRANSPORT_MAX_CLIENTS: usize = 1024;
pub const TRANSPORT_MAX_PENDING_CLIENTS: usize = TRANSPORT_MAX_CLIENTS * 4;

pub const TRANSPORT_MAX_PACKET_BYTES: usize = 1400;
/// The maximum number of bytes that a payload can have when generating a payload packet.
pub const TRANSPORT_MAX_PAYLOAD_BYTES: usize = 1300;
pub const MAX_MESSAGES_LENGTH: usize = 1200;
pub const TRANSPORT_SEND_RATE: Duration = Duration::from_millis(250);

pub static VELOCITY_MUL: f32 = 0.3;
pub static JUMP_SPEED: f32 = 5.5;
pub static GRAVITY: f32 = 9.8;

pub static TICK_DELTA: Duration = Duration::from_millis(16);

pub static DEBUG_CAMERA_SENSITIVITY: f32 = 0.01;

pub static MAIN_SESSION_ID: u32 = 0;
