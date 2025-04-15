use raylib::math::*;
use raylib::prelude::*;
use utils::c_bytesto_string;
use std::f32::consts::FRAC_PI_2;
use std::f32::consts::TAU;

mod utils;

const SCREEN_WIDTH: i32 = 1280;
const SCREEN_HEIGHT: i32 = 800;
const CAMERA_MOVE_SPEED: f32 = 20.0;
const MOUSE_SENSITIVITY: f32 = 0.0015;
const FRICTION: f32 = 5.0;
const GRAVITY: f32 = 9.8;
const JUMP_FORCE: f32 = 5.0;
const GROUND_Y: f32 = 0.0; // Ground level

pub struct Player {
    position: Vector3,
    velocity: Vector3,
    orientation: Vector2, //  x = pitch, y = yaw
    target: Vector3,
    model: Model,
    model_animations: Vec<ModelAnimation>,
    bounding_box: BoundingBox,
    is_grounded: bool,
}


pub struct Map{
    model: Model,
    boundings: Vec<BoundingBox>,
}

fn check_collision(object: &BoundingBox, target: &Vec<BoundingBox>, current_pos: Vector3) -> Vector3 {
    let mut new_pos = current_pos;
    let mut collision_normal = Vector3::zero();
    let mut has_collision = false;

    // Transform object bounding box to world space
    let world_object_min = object.min + current_pos;
    let world_object_max = object.max + current_pos;
    let world_object = BoundingBox {
        min: world_object_min,
        max: world_object_max,
    };

    target.iter().for_each(|bounding| {
        // Transform target bounding box to world space (assuming map is at origin)
        let world_target = *bounding;
        
        if world_object.check_collision_boxes(world_target) {
            has_collision = true;
            // Calculate collision normal (simplified)
            let object_center = (world_object.max + world_object.min) * 0.5;
            let target_center = (world_target.max + world_target.min) * 0.5;
            let center_diff = object_center - target_center;
            collision_normal = center_diff.normalized();
        }
    });

    if has_collision {
        // Push the position back along the collision normal
        new_pos += collision_normal;
    }

    new_pos
}


fn check_collision2(object: &BoundingBox, target: &Vec<BoundingBox>) -> bool {

    target.iter().find(|bounding| {
        object.check_collision_boxes(**bounding)
    }).is_some()

}


pub fn update_player(rl: &RaylibHandle, player: &mut Player, map: &Map) {
    player.orientation.y -= rl.get_mouse_delta().x * MOUSE_SENSITIVITY;
    player.orientation.x += rl.get_mouse_delta().y * MOUSE_SENSITIVITY;
    player.orientation.y = player.orientation.y.rem_euclid(TAU);
    player.orientation.x = player.orientation.x.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);

    let mut rot = Quaternion::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), player.orientation.y) *
     Quaternion::from_axis_angle(Vector3::new(1.0, 0.0, 0.0), player.orientation.x);

    rot = rot.normalized();

    let front = Vector3::new(0.0, 0.0, 1.0).rotate_by(rot);
    let side = Vector3::new(1.0, 0.0, 0.0).rotate_by(rot);

    let dt = rl.get_frame_time();

    // Handle keyboard input
    if rl.is_key_down(KeyboardKey::KEY_W) {
        player.velocity += front * dt * CAMERA_MOVE_SPEED;
    }
    if rl.is_key_down(KeyboardKey::KEY_S) {
        player.velocity -= front * dt * CAMERA_MOVE_SPEED;
    }
    if rl.is_key_down(KeyboardKey::KEY_A) {
        player.velocity += side * dt * CAMERA_MOVE_SPEED;
    }
    if rl.is_key_down(KeyboardKey::KEY_D) {
        player.velocity -= side * dt * CAMERA_MOVE_SPEED;
    }

    // Handle jumping
    if rl.is_key_pressed(KeyboardKey::KEY_SPACE) && player.is_grounded {
        player.velocity.y = JUMP_FORCE;
        player.is_grounded = false;
    }

    // Apply gravity
    if !player.is_grounded {
        player.velocity.y -= GRAVITY * dt;
    }

    // Apply friction
    let friction_factor = 1.0 + dt * FRICTION;
    player.velocity.x /= friction_factor;
    player.velocity.z /= friction_factor;

    let mut new_position = player.position;

    // Try movement in each axis separately
    if player.velocity.x != 0.0 {
        let test_pos = Vector3::new(player.position.x + player.velocity.x * dt, player.position.y, player.position.z);
        let world_box = BoundingBox {
            min: player.bounding_box.min + test_pos,
            max: player.bounding_box.max + test_pos,
        };
        if !check_collision2(&world_box, &map.boundings) {
            new_position.x = test_pos.x;
        }
    }

    if player.velocity.y != 0.0 {
        let test_pos = Vector3::new(new_position.x, player.position.y + player.velocity.y * dt, player.position.z);
        let world_box = BoundingBox {
            min: player.bounding_box.min + test_pos,
            max: player.bounding_box.max + test_pos,
        };
        if !check_collision2(&world_box, &map.boundings) {
            new_position.y = test_pos.y;
        } else {
            // If we hit something while moving up, stop upward movement
            if player.velocity.y > 0.0 {
                player.velocity.y = 0.0;
            }
            // If we hit something while moving down, we're grounded
            if player.velocity.y < 0.0 {
                player.is_grounded = true;
                player.velocity.y = 0.0;
            }
        }
    }

    if player.velocity.z != 0.0 {
        let test_pos = Vector3::new(new_position.x, new_position.y, player.position.z + player.velocity.z * dt);
        let world_box = BoundingBox {
            min: player.bounding_box.min + test_pos,
            max: player.bounding_box.max + test_pos,
        };
        if !check_collision2(&world_box, &map.boundings) {
            new_position.z = test_pos.z;
        }
    }

    // Check if we're on the ground
    if new_position.y <= GROUND_Y + player.bounding_box.min.y {
        new_position.y = GROUND_Y + player.bounding_box.min.y;
        player.is_grounded = true;
        player.velocity.y = 0.0;
    }

    player.position = new_position;
    
    // Update target after collision resolution
    player.target = player.position + front;
}


fn main() {
    // Init raylib
    let (mut rl, thread) = raylib::init()
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Important!")
        .msaa_4x()
        .fullscreen()
        .build();
    rl.set_target_fps(60);
    rl.hide_cursor();
    rl.disable_cursor();

    let map_model = rl.load_model(&thread, "resources/map.glb").unwrap();
    let map = Map {
        boundings: map_model.meshes().iter().map(|mesh| mesh.get_mesh_bounding_box()).collect(),
        model: map_model,
    };

    // Calculate map center
    let map_bounding_box = map.model.get_model_bounding_box();
    let map_center = (map_bounding_box.min + map_bounding_box.max) * 0.5;
    
    let player_model = rl.load_model(&thread, "resources/skye.glb").unwrap();
    let mut player = Player {
        position: Vector3 {
            x: map_center.x + 10.0,
            y: map_center.y + 10.0, // Add 1.0 to spawn above the ground
            z: map_center.z + 10.0,
        },
        velocity: Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        orientation: Vector2 { x: 0.0, y: 0.0 },
        target: Vector3 {
            x: map_center.x,
            y: map_center.y + 1.0,
            z: map_center.z + 1.0, // Look forward
        },
        model_animations: rl.load_model_animations(&thread, "resources/skye.glb").unwrap(),
        bounding_box: player_model.get_model_bounding_box(),
        model: player_model,
        is_grounded: true,
    };
    

    let mut camera = Camera3D::perspective(
        Vector3 {
            x: 0.0,
            y: 2.0,
            z: 4.0,
        },
        Vector3 {
            x: 0.0,
            y: 2.0,
            z: 0.0,
        },
        Vector3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        60.0,
    );


    // Find the head bone index
    let head_bone_index = player.model
        .bones()
        .unwrap()
        .iter()
        .position(|bone| c_bytesto_string(&bone.name).eq("Camera"))
        .unwrap();

    println!("Head bone index: {}", head_bone_index);

    let mut anim_current_frame = 0;

    // Render loop
    while !rl.window_should_close() {
        update_player(&rl, &mut player, &map);

        anim_current_frame = (anim_current_frame + 1)%500;
        rl.update_model_animation(&thread, &mut player.model, &player.model_animations[0], anim_current_frame);

        // Get the current bone transform for the head
        let frame_poses = player.model_animations[0].frame_poses();
        let head_transform = &frame_poses[anim_current_frame as usize][head_bone_index];
        
        // Create rotation quaternion from model's yaw
        let model_rotation = Quaternion::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), player.orientation.y);
        
        // Rotate the head bone's translation by the model's rotation
        let rotated_head_offset = head_transform.translation.rotate_by(model_rotation);
        
        camera.position = player.position + rotated_head_offset;
        camera.target = player.target + rotated_head_offset;

        let mut dhl = rl.begin_drawing(&thread);
        dhl.clear_background(Color::WHITE);

        let mut d3d = dhl.begin_mode3D(&camera);

        // Draw the model at player position
        d3d.draw_model_ex(
            &player.model,
            player.position,
            Vector3::new(0.0, 1.0, 0.0),
            player.orientation.y.to_degrees(),
            Vector3::new(1.0, 1.0, 1.0),
            Color::WHITE
        );


        d3d.draw_model(
            &map.model,
            Vector3::new(0.0, 0.0, 0.0),
            1.0,
            Color::WHITE
        );

        drop(d3d);
        drop(dhl);
    }

}
