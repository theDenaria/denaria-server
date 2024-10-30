use bevy::{
    math::Vec3,
    prelude::{Bundle, Component, Entity, Resource},
};
use std::collections::HashMap;

#[derive(Default, Component)]
pub struct Player {
    pub id: String,
}

#[derive(Debug, Component)]
pub struct VerticalVelocity(pub f32);

#[derive(Debug, Component)]
pub struct MoveInput {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub prev_move: Vec3,
}

#[derive(Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub move_input: MoveInput,
    pub v_velocity: VerticalVelocity,
}

impl Default for PlayerBundle {
    fn default() -> Self {
        PlayerBundle {
            player: Player { id: String::new() },
            move_input: MoveInput {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                prev_move: Vec3::ZERO,
            },
            v_velocity: VerticalVelocity(0.0),
        }
    }
}

#[derive(Resource)]
pub struct PlayerLookup {
    pub map: HashMap<String, Entity>,
}

impl PlayerLookup {
    pub fn new() -> PlayerLookup {
        PlayerLookup {
            map: HashMap::new(),
        }
    }
}
