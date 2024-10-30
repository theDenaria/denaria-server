use std::{
    io::{BufReader, BufWriter, Error},
    path::Path,
    vec,
};

use std::fs::File;

use bevy::{
    prelude::*,
    tasks::{ComputeTaskPool, ParallelSlice},
};
use bevy_rapier3d::prelude::*;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::ecs::{
    components::PlayerLookup,
    events::{DisconnectEvent, JumpEvent, LookEvent, MoveEvent, SpawnEvent},
};

#[derive(Debug, Resource)]
pub struct Tick(pub u16);

pub fn setup(mut commands: Commands) {
    let objects: Vec<LevelObject> = vec![];

    let level_objects = LevelObjects { objects };

    commands.insert_resource(PlayerLookup::new());
    commands.insert_resource(level_objects);
    commands.insert_resource(Tick(0));
    commands.insert_resource(Events::<SpawnEvent>::default());
    commands.insert_resource(Events::<DisconnectEvent>::default());
    commands.insert_resource(Events::<LookEvent>::default());
    commands.insert_resource(Events::<MoveEvent>::default());
    commands.insert_resource(Events::<JumpEvent>::default());
}

pub fn setup_level(par_commands: ParallelCommands, mut level_objects: ResMut<LevelObjects>) {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        let level_objects_vec = get_level_objects().await;
        level_objects.objects = level_objects_vec;
    });
    trace!(
        "Spawning {:?} level object colliders",
        level_objects.objects.len()
    );
    level_objects
        .objects
        .par_splat_map(ComputeTaskPool::get(), None, |_, data| {
            for item in data.iter() {
                par_commands.command_scope(|mut commands| match item.object_type.as_str() {
                    "MeshCollider" => item.new_mesh(&mut commands),
                    "CapsuleCollider" => item.new_capsule(&mut commands),
                    "SphereCollider" => item.new_sphere(&mut commands),
                    "BoxCollider" => item.new_cuboid(&mut commands),
                    _ => {}
                });
            }
        });
    info!("Level Objects setup completed, Denaria Server is ready!!");
}

pub async fn get_level_objects() -> Vec<LevelObject> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true) // Ignore SSL certificate validation
        .build()
        .expect("Failed to create client");
    let mut level_objects: Vec<LevelObject> = vec![];

    dotenvy::dotenv().unwrap();

    let level_server_url = std::env::var("LEVEL_SERVER").unwrap();
    let level_objects_version = std::env::var("LEVEL_OBJECTS_VERSION").unwrap();

    let get_first_id_url = format!(
        "{}/get-first?version={}",
        level_server_url.as_str(),
        level_objects_version.as_str()
    );

    let first_id_res = client.get(&get_first_id_url).send().await.unwrap();
    let first_id: LevelObjectFirstIdResponse = first_id_res.json().await.unwrap();

    let mut i = first_id.id;

    let level_cache_file_path = format!(
        "level_objects_cache_v{}.json",
        level_objects_version.as_str()
    );

    let cached_level_objects: Vec<LevelObject> = match read_from_file(&level_cache_file_path) {
        Ok(o) => o,
        Err(_) => Vec::new(),
    };

    let mut use_cache = false;

    if cached_level_objects.len() > 0 {
        if cached_level_objects[0].id == i {
            tracing::info!("Using cache for level objects");
            use_cache = true;
            level_objects = cached_level_objects;
        }
    }
    if !use_cache {
        info!("Valid chace not found, getting level objects from level-server");
        loop {
            let url = format!(
                "{}/get-object?version={}&id={}",
                level_server_url.as_str(),
                level_objects_version.as_str(),
                i
            );

            // Use the blocking client to make a synchronous request
            let res = client.get(&url).send().await.unwrap();

            if res.status().is_success() {
                tracing::trace!("Object {} fetched", i);
            } else {
                tracing::trace!("Failed to fetch object {}", i);
                break;
            }

            let object_db: LevelObjectSchema = res.json().await.unwrap();

            let position: Vector3Deserialized =
                serde_json::from_str(object_db.position.as_str()).unwrap();
            let rotation: Vector4Deserialized =
                serde_json::from_str(object_db.rotation.as_str()).unwrap();
            let scale: Vector3Deserialized =
                serde_json::from_str(object_db.scale.as_str()).unwrap();

            let object_type = object_db.object_type;

            let translation = Vec3::new(position.x, position.y, position.z);
            let rotation = Quat::from_xyzw(rotation.x, rotation.y, rotation.z, rotation.w);

            let scale = Vec3::new(scale.x, scale.y, scale.z);

            let level_object = LevelObject {
                id: object_db.id,
                object_type,
                translation,
                rotation,
                scale,
                collider: object_db.collider,
            };

            level_objects.push(level_object);

            i += 1;
        }

        write_to_file(&level_cache_file_path, &level_objects).unwrap();
        info!(
            "Latest level objects are cached to file: {}",
            level_cache_file_path.as_str()
        )
    }

    level_objects
}

#[derive(Debug, Resource, Serialize)]
pub struct LevelObjects {
    objects: Vec<LevelObject>,
}

// Level Object size format uses the convention of Unity3D Game Engine's scale
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelObject {
    // Ball: 0, Cube: 1, Capsule: 2
    id: i32,
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
pub struct LevelObjectFirstIdResponse {
    id: i32,
}

// Level Object size format uses the convention of Unity3D Game Engine's scale
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelObjectSchema {
    id: i32,
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

        commands
            .spawn(RigidBody::Fixed)
            .insert(Collider::trimesh(vertices, indices))
            .insert(TransformBundle::from(
                Transform::from_xyz(self.translation.x, self.translation.y, self.translation.z)
                    .with_rotation(self.rotation)
                    .with_scale(self.scale),
            ));
    }
}

// Function to read data from a file
fn read_from_file(file_path: &str) -> Result<Vec<LevelObject>, Error> {
    let path = Path::new(file_path);
    if path.exists() {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let level_objects: Vec<LevelObject> = serde_json::from_reader(reader)?;
        Ok(level_objects)
    } else {
        Ok(Vec::new())
    }
}

// Function to write data to a file
fn write_to_file(file_path: &str, data: &Vec<LevelObject>) -> Result<(), Error> {
    // let json_data = serde_json::to_string_pretty(data)?;
    // let mut file = File::create(file_path)?;
    // file.write_all(json_data.as_bytes())?;
    let file = File::create(file_path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, data)?;
    Ok(())
}
