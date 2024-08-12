use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
    vec,
};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::{
    ecs::{
        components::PlayerLookup,
        events::{ConnectEvent, DisconnectEvent, FireEvent, HitEvent, LookEvent},
    },
    server::{
        channel::DefaultChannel,
        connection::ConnectionConfig,
        message_out::MessageOut,
        server::MattaServer,
        transport::{server::server::ServerConfig, transport::ServerTransport},
    },
};

pub fn setup(mut commands: Commands) {
    let server = MattaServer::new(ConnectionConfig::default());
    // Setup transport layer
    const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
    let socket: UdpSocket = UdpSocket::bind(SERVER_ADDR).unwrap();
    let server_config = ServerConfig {
        current_time: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        max_clients: 64,
        public_addresses: vec![SERVER_ADDR],
    };
    let transport = ServerTransport::new(server_config, socket).unwrap();

    let objects: Vec<LevelObject> = vec![];

    let level_objects = LevelObjects { objects };

    commands.insert_resource(server);
    commands.insert_resource(transport);
    commands.insert_resource(PlayerLookup::new());
    commands.insert_resource(level_objects);

    commands.insert_resource(Events::<ConnectEvent>::default());
    commands.insert_resource(Events::<DisconnectEvent>::default());
    commands.insert_resource(Events::<LookEvent>::default());
    commands.insert_resource(Events::<FireEvent>::default());
    commands.insert_resource(Events::<HitEvent>::default());
}

pub fn setup_level(mut commands: Commands, mut level_objects: ResMut<LevelObjects>) {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        let level_objects_vec = get_level_objects().await;
        level_objects.objects = level_objects_vec;
    });
    trace!("Before");
    for object in level_objects.objects.iter() {
        match object.object_type.as_str() {
            "MeshCollider" => object.new_mesh(&mut commands),
            "CapsuleCollider" => object.new_capsule(&mut commands),
            "SphereCollider" => object.new_sphere(&mut commands),
            "BoxCollider" => object.new_cuboid(&mut commands),
            _ => {}
        }
    }
}

pub async fn get_level_objects() -> Vec<LevelObject> {
    let mut i = 2667;
    let client = Client::new();

    let mut level_objects: Vec<LevelObject> = vec![];

    loop {
        // if i > 2143 {
        //     break;
        // }
        let url = format!("http://localhost:3000/get-object?id={}", i);

        // Use the blocking client to make a synchronous request
        let res = client.get(&url).send().await.unwrap();

        if res.status().is_success() {
        } else {
            println!("Failed to fetch object {}", i);
            break;
        }

        let object_db: LevelObjectSchema = res.json().await.unwrap();

        let position: Vector3Deserialized =
            serde_json::from_str(object_db.position.as_str()).unwrap();
        let rotation: Vector4Deserialized =
            serde_json::from_str(object_db.rotation.as_str()).unwrap();
        let scale: Vector3Deserialized = serde_json::from_str(object_db.scale.as_str()).unwrap();

        let object_type = object_db.object_type;

        let translation = Vec3::new(position.x, position.y, position.z);
        let rotation = Quat::from_xyzw(rotation.x, rotation.y, rotation.z, rotation.w);

        let scale = Vec3::new(scale.x, scale.y, scale.z);

        let level_object = LevelObject {
            object_type,
            translation,
            rotation,
            scale,
            collider: object_db.collider,
        };

        level_objects.push(level_object);

        i += 1;
    }
    level_objects
}

pub fn send_level_objects(
    server: &mut MattaServer,
    level_objects: &LevelObjects,
    player_id: String,
) {
    let level_objects_message =
        MessageOut::level_objects_message(level_objects.objects.clone()).unwrap();

    let client_id = server.client_id_by_player_id(player_id).unwrap();

    server.send_message(
        client_id,
        DefaultChannel::ReliableOrdered,
        level_objects_message.data,
    )
}

#[derive(Debug, Resource, Serialize)]
pub struct LevelObjects {
    objects: Vec<LevelObject>,
}

// Level Object size format uses the convention of Unity3D Game Engine's scale
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelObject {
    // Ball: 0, Cube: 1, Capsule: 2
    //id: i32,
    object_type: String,
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
    collider: String,
}

#[derive(Deserialize)]
struct Vector3Deserialized {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Deserialize)]
struct Vector4Deserialized {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

// Level Object size format uses the convention of Unity3D Game Engine's scale
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelObjectSchema {
    object_type: String,
    position: String,
    rotation: String,
    scale: String,
    collider: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MeshData {
    vertices: Vec<Vec3>,
    triangles: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CuboidData {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct BallData {
    radius: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CapsuleData {
    radius: f32,
    height: f32,
    direction: i32,
}

#[derive(Deserialize)]
struct VerticeDeserialized {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Deserialize)]
struct MeshDataDeserialized {
    vertices: Vec<VerticeDeserialized>,
    triangles: Vec<i32>,
}

impl MeshDataDeserialized {
    fn to_mesh_data(&self) -> MeshData {
        MeshData {
            vertices: self
                .vertices
                .iter()
                .map(|ver| Vec3::new(ver.x, ver.y, ver.z))
                .collect(),
            triangles: self.triangles.clone(),
        }
    }
}

impl LevelObject {
    fn new_cuboid(&self, commands: &mut Commands) {
        let coboid_data: CuboidData = serde_json::from_str(self.collider.as_str()).unwrap();
        commands
            .spawn(RigidBody::Fixed)
            .insert(Collider::cuboid(
                coboid_data.x / 2.0,
                coboid_data.y / 2.0,
                coboid_data.z / 2.0,
            ))
            .insert(TransformBundle::from(
                Transform::from_translation(self.translation)
                    .with_rotation(self.rotation)
                    .with_scale(self.scale),
            ));
    }

    fn new_capsule(&self, commands: &mut Commands) {
        let capsule_data: CapsuleData = serde_json::from_str(self.collider.as_str()).unwrap();
        match capsule_data.direction {
            0 => {
                commands
                    .spawn(RigidBody::Fixed)
                    .insert(Collider::capsule_x(
                        capsule_data.height / 2.0,
                        capsule_data.radius,
                    ))
                    .insert(TransformBundle::from(
                        Transform::from_translation(self.translation)
                            .with_rotation(self.rotation)
                            .with_scale(self.scale),
                    ));
            }

            1 => {
                commands
                    .spawn(RigidBody::Fixed)
                    .insert(Collider::capsule_y(
                        capsule_data.height / 2.0,
                        capsule_data.radius,
                    ))
                    .insert(TransformBundle::from(
                        Transform::from_translation(self.translation)
                            .with_rotation(self.rotation)
                            .with_scale(self.scale),
                    ));
            }

            2 => {
                commands
                    .spawn(RigidBody::Fixed)
                    .insert(Collider::capsule_z(
                        capsule_data.height / 2.0,
                        capsule_data.radius,
                    ))
                    .insert(TransformBundle::from(
                        Transform::from_translation(self.translation)
                            .with_rotation(self.rotation)
                            .with_scale(self.scale),
                    ));
            }
            _ => {
                tracing::error!("Invalid Capsule collider direction");
                return;
            }
        };
    }

    fn new_sphere(&self, commands: &mut Commands) {
        let ball_data: BallData = serde_json::from_str(self.collider.as_str()).unwrap();
        commands
            .spawn(RigidBody::Fixed)
            .insert(Collider::ball(ball_data.radius))
            .insert(TransformBundle::from(
                Transform::from_translation(self.translation)
                    .with_rotation(self.rotation)
                    .with_scale(self.scale),
            ));
    }

    fn new_mesh(&self, commands: &mut Commands) {
        let data: MeshDataDeserialized = serde_json::from_str(self.collider.as_str()).unwrap();
        let mesh_data = data.to_mesh_data();

        let vertices = mesh_data
            .vertices
            .iter()
            .map(|vertice| Vec3::new(vertice.x, vertice.y, vertice.z))
            .collect();

        let indices: Vec<[u32; 3]> = mesh_data
            .triangles
            .chunks(3)
            .map(|chunk| [chunk[0] as u32, chunk[1] as u32, chunk[2] as u32])
            .collect();

        let id = commands
            .spawn(RigidBody::Fixed)
            .insert(Collider::trimesh(vertices, indices))
            .insert(TransformBundle::from(
                Transform::from_xyz(self.translation.x, self.translation.y, self.translation.z)
                    .with_rotation(self.rotation)
                    .with_scale(self.scale),
            ))
            .id();

        trace!("Collider spawned by id : {:?}", id);
    }
}