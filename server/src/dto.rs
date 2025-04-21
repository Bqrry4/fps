use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerInfo {
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub yaw: f32,
    pub pitch: f32,
}