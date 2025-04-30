use serde::{Serialize, Deserialize};


//Normally i should declare some communication protocol to optimize the server workflow, broadcasting data cuz no time to
#[derive(Serialize, Deserialize, Debug)]
pub struct PlayerInfo {
    pub id: i32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub skin: String,
}