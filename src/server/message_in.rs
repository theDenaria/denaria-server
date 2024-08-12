use bevy::math::Vec3;

#[derive(Debug)]
pub struct MessageIn {
    pub event_type: MessageInType,
    pub data: Vec<u8>,
}

impl MessageIn {
    pub fn new(bytes: Vec<u8>) -> Result<MessageIn, &'static str> {
        if bytes.len() < 1 {
            return Err("Not enough bytes for EventIn");
        }

        let event_type = MessageInType::try_from(bytes[0]).map_err(|_| "Invalid event type")?;
        let data = &bytes[1..];

        Ok(MessageIn {
            event_type,
            data: data.to_vec(),
        })
    }
}

#[derive(Debug)]
pub enum MessageInType {
    Connect = 0,
    Move = 2,
    Rotation = 3,
    Jump = 4,
    Fire = 5,
    Invalid = 99,
}

#[derive(Debug)]
pub struct MoveMessageIn {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct RotationMessageIn {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Debug)]
pub struct FireMessageIn {
    pub cam_origin: Vec3,
    pub direction: Vec3,
    pub barrel_origin: Vec3,
}

pub struct Jump {}

#[derive(Debug)]
pub struct ConnectMessageIn {
    pub message: String,
}

#[derive(Debug)]
pub struct DisconnectMessageIn {
    pub message: String,
}

impl TryFrom<u8> for MessageInType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MessageInType::Connect),
            2 => Ok(MessageInType::Move),
            3 => Ok(MessageInType::Rotation),
            4 => Ok(MessageInType::Jump),
            5 => Ok(MessageInType::Fire),
            _ => Ok(MessageInType::Invalid),
        }
    }
}

pub fn digest_move_message(data: Vec<u8>) -> Result<MoveMessageIn, &'static str> {
    if data.len() < 8 {
        println!("Insufficent bytes: {:?}", data);
        return Err("Insufficient bytes for MoveInputUpdate");
    }

    let x_bytes = data[0..4]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;
    let y_bytes = data[4..8]
        .try_into()
        .map_err(|_| "Failed to slice y bytes")?;

    let x = f32::from_ne_bytes(x_bytes);
    let y = f32::from_ne_bytes(y_bytes);

    Ok(MoveMessageIn { x, y })
}

pub fn digest_rotation_message(data: Vec<u8>) -> Result<RotationMessageIn, &'static str> {
    if data.len() < 12 {
        println!("Insufficent bytes: {:?}", data);
        return Err("Insufficient bytes for Rotation");
    }

    let rotation_bytes_x = data[0..4]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;
    let rotation_bytes_y = data[4..8]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;
    let rotation_bytes_z = data[8..12]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;
    let rotation_bytes_w = data[12..16]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;

    let rotation_x = f32::from_ne_bytes(rotation_bytes_x);
    let rotation_y = f32::from_ne_bytes(rotation_bytes_y);
    let rotation_z = f32::from_ne_bytes(rotation_bytes_z);
    let rotation_w = f32::from_ne_bytes(rotation_bytes_w);

    Ok(RotationMessageIn {
        x: rotation_x,
        y: rotation_y,
        z: rotation_z,
        w: rotation_w,
    })
}

pub fn digest_connect_message(data: Vec<u8>) -> Result<ConnectMessageIn, &'static str> {
    if data.len() < 1 {
        return Err("Insufficient bytes for Connect");
    }

    let message = String::from_utf8(data).map_err(|_| "Invalid UTF-8 in player_id")?;

    Ok(ConnectMessageIn { message })
}

pub fn digest_fire_message(data: Vec<u8>) -> Result<FireMessageIn, &'static str> {
    if data.len() < 8 {
        println!("Insufficent bytes: {:?}", data);
        return Err("Insufficient bytes for MoveInputUpdate");
    }

    let cam_origin_x_bytes = data[0..4]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;
    let cam_origin_y_bytes = data[4..8]
        .try_into()
        .map_err(|_| "Failed to slice y bytes")?;
    let cam_origin_z_bytes = data[8..12]
        .try_into()
        .map_err(|_| "Failed to slice y bytes")?;

    let direction_x_bytes = data[12..16]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;
    let direction_y_bytes = data[16..20]
        .try_into()
        .map_err(|_| "Failed to slice y bytes")?;
    let direction_z_bytes = data[20..24]
        .try_into()
        .map_err(|_| "Failed to slice y bytes")?;

    let barrel_origin_x_bytes = data[24..28]
        .try_into()
        .map_err(|_| "Failed to slice x bytes")?;
    let barrel_origin_y_bytes = data[28..32]
        .try_into()
        .map_err(|_| "Failed to slice y bytes")?;
    let barrel_origin_z_bytes = data[32..36]
        .try_into()
        .map_err(|_| "Failed to slice y bytes")?;

    let cam_origin = Vec3::new(
        f32::from_ne_bytes(cam_origin_x_bytes),
        f32::from_ne_bytes(cam_origin_y_bytes),
        f32::from_ne_bytes(cam_origin_z_bytes),
    );

    let direction = Vec3::new(
        f32::from_ne_bytes(direction_x_bytes),
        f32::from_ne_bytes(direction_y_bytes),
        f32::from_ne_bytes(direction_z_bytes),
    );

    let barrel_origin = Vec3::new(
        f32::from_ne_bytes(barrel_origin_x_bytes),
        f32::from_ne_bytes(barrel_origin_y_bytes),
        f32::from_ne_bytes(barrel_origin_z_bytes),
    );

    Ok(FireMessageIn {
        cam_origin,
        direction,
        barrel_origin,
    })
}
