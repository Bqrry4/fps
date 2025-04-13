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
const FRICTION: f32 = 10.0;


pub struct Player {
    position: Vector3,
    velocity: Vector3,
    orientation: Vector2, //  x = pitch, y = yaw
    target: Vector3,
}

pub fn update_player(rl: &RaylibHandle, player: &mut Player) {

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
    if rl.is_key_down(KeyboardKey::KEY_SPACE) {
        player.velocity += CAMERA_MOVE_SPEED;
    }
    if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) {
        player.velocity -= CAMERA_MOVE_SPEED;
    }

    // Apply friction
    player.velocity /= 1.0 + dt * FRICTION;


    player.position += player.velocity * dt;
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

    let mut player = Player {
        position: Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        velocity: Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        orientation: Vector2 { x: 0.0, y: 0.0 },
        target: Vector3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
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

    //let map = rl.load_model(&thread, "resources/cs_office_with_real_light.glb").unwrap();

    let mut model = rl.load_model(&thread, "resources/test.glb").unwrap();
    let model_animations = rl.load_model_animations(&thread, "resources/test.glb").unwrap();
    
    model.bind_pose();

    // // Find the head bone index
    // let head_bone_index = model
    //     .bones()
    //     .unwrap()
    //     .iter()
    //     .position(|bone| c_bytesto_string(&bone.name).contains("Head"))
    //     .unwrap();

    // println!("Head bone index: {}", head_bone_index);

    // // Get the animation and its frame poses
    // let anim = &model_animations[0];
    // let frame_poses = anim.frame_poses();
    let mut anim_current_frame = 0;

    // println!("Anim frame count: {}", anim.frameCount);


    // Render loop
    while !rl.window_should_close() {
        update_player(&rl, &mut player);

        anim_current_frame = (anim_current_frame + 1)%500;
        rl.update_model_animation(&thread, &mut model, &model_animations[0], anim_current_frame);

        // Get the current bone transform
        // let bone_transform = &frame_poses[0][head_bone_index];
        
        // Update camera position based on bone transform
        camera.position = player.position ;
        camera.target = player.target ;

        let mut dhl = rl.begin_drawing(&thread);
        dhl.clear_background(Color::WHITE);

        let mut d3d = dhl.begin_mode3D(&camera);

        // Draw the model at player position
        d3d.draw_model_ex(
            &model,
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            // player.orientation.y.to_degrees(),
            0.0,
            Vector3::new(1.0, 1.0, 1.0),
            Color::WHITE
        );

        // d3d.draw_model(
        //     &map,
        //     Vector3::new(0.0, 0.0, 0.0),
        //     1.0,
        //     Color::WHITE
        // );

        drop(d3d);
        drop(dhl);
    }

}
