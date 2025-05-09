use anchor_client::anchor_lang::AnchorDeserialize;
use anchor_client::anchor_lang::solana_program;
use anchor_client::anchor_lang::solana_program::pubkey;
use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_client::rpc_request::TokenAccountsFilter;
use anchor_client::solana_sdk::pubkey::Pubkey;
use mpl_token_metadata::accounts::Metadata;

use sol_client::TextureField;
use solana_account_decoder::UiAccountData;

use reqwest::blocking::get;
use serde_json::Value;

mod sol_client;
use sol_client::SkinMetadata;
use sol_client::SolanaClient;

use raylib::math::*;
use raylib::prelude::*;
use std::collections::HashMap;
use std::env;
use std::f32::consts::FRAC_PI_2;
use std::f32::consts::PI;
use std::f32::consts::TAU;
use std::io;
use std::io::stdin;
use std::mem::swap;
use std::str::FromStr;
use std::sync::atomic::Ordering;

mod utils;
use utils::c_bytesto_string;

mod net_client;
use net_client::NetworkClient;

mod dto;
use dto::PlayerInfo;

mod m_player;
use m_player::MPlayer;

const SCREEN_WIDTH: i32 = 1280;
const SCREEN_HEIGHT: i32 = 800;
const CAMERA_MOVE_SPEED: f32 = 0.4;
const MOUSE_SENSITIVITY: f32 = 0.0015;
const FRICTION: f32 = 5.0;
const GRAVITY: f32 = 9.8;
const JUMP_FORCE: f32 = 7.5;
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

pub struct Map {
    model: Model,
    boundings: Vec<BoundingBox>,
}

fn check_collision(object: &BoundingBox, target: &Vec<BoundingBox>) -> bool {
    target
        .iter()
        .find(|bounding| object.check_collision_boxes(**bounding))
        .is_some()
}

pub fn update_player(rl: &RaylibHandle, player: &mut Player, map: &Map) {
    player.orientation.y -= rl.get_mouse_delta().x * MOUSE_SENSITIVITY;
    player.orientation.x += rl.get_mouse_delta().y * MOUSE_SENSITIVITY;
    player.orientation.y = player.orientation.y.rem_euclid(TAU);
    player.orientation.x = player
        .orientation
        .x
        .clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);

    let mut rot = Quaternion::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), player.orientation.y)
        * Quaternion::from_axis_angle(Vector3::new(1.0, 0.0, 0.0), player.orientation.x);

    rot = rot.normalized();

    let front = Vector3::new(0.0, 0.0, 1.0).rotate_by(rot);
    let side = Vector3::new(1.0, 0.0, 0.0).rotate_by(rot);

    let dt = rl.get_frame_time();

    let mut movement = Vector3::zero();
    // Handle keyboard input
    if rl.is_key_down(KeyboardKey::KEY_W) {
        movement += front;
    }
    if rl.is_key_down(KeyboardKey::KEY_S) {
        movement -= front;
    }
    if rl.is_key_down(KeyboardKey::KEY_A) {
        movement += side;
    }
    if rl.is_key_down(KeyboardKey::KEY_D) {
        movement -= side;
    }
    movement *= CAMERA_MOVE_SPEED;

    let mut velocity = player.velocity;
    velocity.x += movement.x;
    velocity.z += movement.z;

    // Try movement in each axis separately
    if velocity.x != 0.0 {
        let test_pos = Vector3::new(
            player.position.x + velocity.x * dt,
            player.position.y,
            player.position.z,
        );
        let world_box = BoundingBox {
            min: player.bounding_box.min + test_pos,
            max: player.bounding_box.max + test_pos,
        };
        if !check_collision(&world_box, &map.boundings) {
            player.position.x = test_pos.x;
            player.velocity.x = velocity.x;
        }
    }

    if velocity.y != 0.0 {
        let test_pos = Vector3::new(
            player.position.x,
            player.position.y + velocity.y * dt,
            player.position.z,
        );
        let world_box = BoundingBox {
            min: player.bounding_box.min + test_pos,
            max: player.bounding_box.max + test_pos,
        };

        if !check_collision(&world_box, &map.boundings) {
            player.position.y = test_pos.y;
            player.velocity.y = velocity.y;
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

    if velocity.z != 0.0 {
        let test_pos = Vector3::new(
            player.position.x,
            player.position.y,
            player.position.z + velocity.z * dt,
        );
        let world_box = BoundingBox {
            min: player.bounding_box.min + test_pos,
            max: player.bounding_box.max + test_pos,
        };
        if !check_collision(&world_box, &map.boundings) {
            player.position.z = test_pos.z;
            player.velocity.z = velocity.z;
        }
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

    // // Check if we're on the ground
    let ground_level = GROUND_Y + player.bounding_box.min.y;
    if player.position.y < ground_level {
        player.position.y = ground_level;
        player.velocity.y = 0.0;
        player.is_grounded = true;
    }

    // player.position = new_position;

    // Update target after collision resolution
    player.target = player.position + front;
}

fn draw_bounding_box(d3d: &mut RaylibMode3D<RaylibDrawHandle>, bbox: &BoundingBox, color: Color) {
    let min = bbox.min;
    let max = bbox.max;

    // Draw box edges
    d3d.draw_line3D(
        Vector3::new(min.x, min.y, min.z),
        Vector3::new(max.x, min.y, min.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(min.x, min.y, min.z),
        Vector3::new(min.x, max.y, min.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(min.x, min.y, min.z),
        Vector3::new(min.x, min.y, max.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(max.x, max.y, max.z),
        Vector3::new(min.x, max.y, max.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(max.x, max.y, max.z),
        Vector3::new(max.x, min.y, max.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(max.x, max.y, max.z),
        Vector3::new(max.x, max.y, min.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(min.x, max.y, min.z),
        Vector3::new(max.x, max.y, min.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(max.x, min.y, min.z),
        Vector3::new(max.x, max.y, min.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(min.x, min.y, max.z),
        Vector3::new(max.x, min.y, max.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(min.x, max.y, min.z),
        Vector3::new(min.x, max.y, max.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(min.x, min.y, max.z),
        Vector3::new(min.x, max.y, max.z),
        color,
    );
    d3d.draw_line3D(
        Vector3::new(max.x, min.y, max.z),
        Vector3::new(max.x, max.y, max.z),
        color,
    );
}
// Assigns weak_textures to materials, must be unloaded manually
pub fn load_hands(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    shader: &Shader,
    gun_textures: &HashMap<String, WeakTexture2D>,
) -> (Model, Vec<ModelAnimation>) {
    let mut hands = rl
        .load_model(&thread, "resources/fps_ak.glb")
        .expect("Could not load resources/fps_ak.glb");
    let hands_animations = rl
        .load_model_animations(&thread, "resources/fps_ak.glb")
        .expect("Could not load animations for resources/fps_ak.glb");

    // let ak74_color = unsafe {
    //     rl.load_texture(&thread, "resources/textures/ak/ak_a.png")
    //         .expect("Failed to load ak_a.png")
    //         .make_weak()
    // };
    // let ak74_normal = unsafe {
    //     rl.load_texture(&thread, "resources/textures/ak/ak_n.png")
    //         .expect("Failed to load ak_n.png")
    //         .make_weak()
    // };
    // let ak_ao = unsafe {
    //     rl.load_texture(&thread, "resources/textures/ak/ak_ao.png")
    //         .expect("Failed to load ak_ao.png")
    //         .make_weak()
    // };
    // let ak_metallic = unsafe {
    //     rl.load_texture(&thread, "resources/textures/ak/ak_m.png")
    //         .expect("Failed to load ak_m.png")
    //         .make_weak()
    // };
    // let ak_roughness = unsafe {
    //     rl.load_texture(&thread, "resources/textures/ak/ak_r.png")
    //         .expect("Failed to load ak_r.png")
    //         .make_weak()
    // };
    let arm_color = unsafe {
        rl.load_texture(&thread, "resources/textures/arm/armColor.png")
            .expect("Failed to load armColor.png")
            .make_weak()
    };
    let arm_normal = unsafe {
        rl.load_texture(&thread, "resources/textures/arm/armNormal.png")
            .expect("Failed to load armNormal.png")
            .make_weak()
    };
    let arm_roughness = unsafe {
        rl.load_texture(&thread, "resources/textures/arm/armRoughness.png")
            .expect("Failed to load armRoughness.png")
            .make_weak()
    };

    let material = &mut hands.materials_mut()[1];
    material.set_material_texture(MaterialMapIndex::MATERIAL_MAP_ALBEDO, &gun_textures["a"]);
    material.set_material_texture(MaterialMapIndex::MATERIAL_MAP_METALNESS, &gun_textures["m"]);
    material.set_material_texture(MaterialMapIndex::MATERIAL_MAP_NORMAL, &gun_textures["n"]);
    material.set_material_texture(MaterialMapIndex::MATERIAL_MAP_ROUGHNESS, &gun_textures["r"]);
    material.set_material_texture(
        MaterialMapIndex::MATERIAL_MAP_OCCLUSION,
        &gun_textures["ao"],
    );
    material.shader = (*shader).clone();

    let material = &mut hands.materials_mut()[2];
    material.set_material_texture(MaterialMapIndex::MATERIAL_MAP_ALBEDO, arm_color);
    material.set_material_texture(MaterialMapIndex::MATERIAL_MAP_NORMAL, arm_normal);
    material.set_material_texture(MaterialMapIndex::MATERIAL_MAP_ROUGHNESS, arm_roughness);
    material.shader = (*shader).clone();

    (hands, hands_animations)
}

pub fn load_lighting_shader(rl: &mut RaylibHandle, thread: &RaylibThread) -> Shader {
    let mut shader = rl.load_shader(
        &thread,
        Some("resources/shaders/pbr.vs"),
        Some("resources/shaders/pbr.fs"),
    );
    shader.locs_mut()[ShaderLocationIndex::SHADER_LOC_MAP_ALBEDO as usize] =
        shader.get_shader_location("albedoMap");
    shader.locs_mut()[ShaderLocationIndex::SHADER_LOC_MAP_METALNESS as usize] =
        shader.get_shader_location("mMap");
    shader.locs_mut()[ShaderLocationIndex::SHADER_LOC_MAP_ROUGHNESS as usize] =
        shader.get_shader_location("rMap");
    shader.locs_mut()[ShaderLocationIndex::SHADER_LOC_MAP_OCCLUSION as usize] =
        shader.get_shader_location("aoMap");
    shader.locs_mut()[ShaderLocationIndex::SHADER_LOC_MAP_NORMAL as usize] =
        shader.get_shader_location("normalMap");
    shader.locs_mut()[ShaderLocationIndex::SHADER_LOC_MAP_EMISSION as usize] =
        shader.get_shader_location("emissiveMap");
    shader.locs_mut()[ShaderLocationIndex::SHADER_LOC_COLOR_DIFFUSE as usize] =
        shader.get_shader_location("albedoColor");
    shader.locs_mut()[ShaderLocationIndex::SHADER_LOC_VECTOR_VIEW as usize] =
        shader.get_shader_location("viewPos");

    // let ambient_intensity = 0.3;
    // let ambient_color = Vector4 {
    //     x: 26.0 / 255.0,
    //     y: 32.0 / 255.0,
    //     z: 135.0 / 255.0,
    //     w: 1.0,
    // };
    let ambient_intensity = 0.6;
    let ambient_color = Vector4 {
        x:  64.0 / 255.0,
        y:  64.0 / 255.0,
        z: 200.0 / 255.0,
        w: 1.0,
    };

    let ambient_loc = shader.get_shader_location("ambientColor");
    let ambient_int_loc = shader.get_shader_location("ambient");
    shader.set_shader_value(ambient_loc, ambient_color);
    shader.set_shader_value(ambient_int_loc, ambient_intensity);

    shader.set_shader_value(shader.get_shader_location("useTexAlbedo"), 1);
    shader.set_shader_value(shader.get_shader_location("useTexNormal"), 1);
    shader.set_shader_value(shader.get_shader_location("useTexMRA"), 1);
    shader.set_shader_value(shader.get_shader_location("useTexEmissive"), 1);

    shader
}

pub fn unload_textures_from_model(rl: &mut RaylibHandle, thread: &RaylibThread, model: &Model) {
    //Force unload textures
    model.materials().iter().for_each(|material| {
        material.maps().iter().for_each(|map| {
            unsafe { rl.unload_texture(&thread, map.texture().to_owned()) };
        });
    });
}

pub fn handle_prompt(skins: &Vec<(Pubkey, SkinMetadata)>) -> &(Pubkey, SkinMetadata) {
    println!("Select a skin:");
    // Display the skin options
    for (index, option) in skins.iter().enumerate() {
        println!("{}: {}/{}", index + 1, option.1.name, option.1.symbol);
    }

    // Read user input
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).expect("Failed to read line");

        match input.trim().parse::<usize>() {
            Ok(choice) if choice > 0 && choice <= skins.len() => {
                let selected_option = &skins[choice - 1];
                return selected_option;
            }
            _ => {
                println!("Invalid input");
            }
        }
    }
}

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <wallet_address>", args[0]);
        return;
    }
    let mut sol_client = SolanaClient::new();
    let pubkey = Pubkey::from_str(&args[1]).unwrap();
    let skins: Vec<(Pubkey, SkinMetadata)> = sol_client.fetch_skins(pubkey).unwrap();

    if skins.len() == 0 {
        println!("No skins found");
        return;
    }

    let choosen_skin = handle_prompt(&skins);

    // Init raylib
    let (mut rl, thread) = raylib::init()
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("Important!")
        .msaa_4x()
        // .fullscreen()
        .build();
    rl.set_target_fps(60);
    rl.hide_cursor();
    rl.disable_cursor();

    //-! Can load some default texs if there are no skins on chain
    //Fetch gun textures
    let gun_texures_bytes = SolanaClient::fetch_images_bytes(&choosen_skin.1.textures).unwrap();
    let gun_textures = SolanaClient::fetch_textures(&mut rl, &thread, gun_texures_bytes)
        .expect("Failed to fetch textures");

    let mut shader = load_lighting_shader(&mut rl, &thread);
    let view_pos_loc = shader.get_shader_location("viewPos");

    let map_model = rl.load_model(&thread, "resources/map.glb").unwrap();
    let mut map = Map {
        boundings: map_model
            .meshes()
            .iter()
            .map(|mesh| mesh.get_mesh_bounding_box())
            .collect(),
        model: map_model,
    };

    // Apply shader to map model
    for i in 0..map.model.materials().len() {
        let material = &mut map.model.materials_mut()[i];
        material.shader = shader.clone();
    }

    // Calculate map center
    let map_bounding_box = map.model.get_model_bounding_box();
    let map_center = (map_bounding_box.min + map_bounding_box.max) * 0.5;

    let mut m_player = MPlayer::load(&mut rl, &thread, &shader).unwrap();

    //This is the T pose bounding box as raylib does not count transforms from animation
    let mut player_box = m_player.model.get_model_bounding_box();
    //-! Little bug with this model, y is switched
    swap(&mut player_box.min.y, &mut player_box.max.y);

    let (mut hands, hands_animations) = load_hands(&mut rl, &thread, &shader, &gun_textures);

    println!("Map center: {:?}", map_center);
    let mut player = Player {
        position: Vector3 {
            x: map_center.x + 10.0,
            y: map_center.y + 10.0,
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
        model_animations: hands_animations,
        bounding_box: player_box, //Take the "generic" box
        model: hands,
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
    let head_bone_index = player
        .model
        .bones()
        .unwrap()
        .iter()
        .position(|bone| c_bytesto_string(&bone.name).eq("Head_Cam"))
        .unwrap();

    println!("Head bone index: {}", head_bone_index);

    let mut anim_current_frame = 0;

    let frame_poses = player.model_animations[0].frame_poses();
    let head_transform = frame_poses[anim_current_frame as usize][head_bone_index];

    //Shift the model origin to the head bone
    player.model.set_transform(&Matrix::translate(
        -head_transform.translation.x,
        -head_transform.translation.y,
        -head_transform.translation.z,
    ));

    //Connect to the server
    let mut net_client = NetworkClient::new().unwrap();
    net_client.connect().unwrap();

    // Render loop
    while !rl.window_should_close() {
        update_player(&rl, &mut player, &map);

        net_client.update();
        net_client.send_update(PlayerInfo {
            id: net_client::CLIENT_ID.load(Ordering::SeqCst),
            position_x: player.position.x,
            position_y: player.position.y,
            position_z: player.position.z,
            yaw: player.orientation.y,
            pitch: player.orientation.x,
            skin: choosen_skin.0.to_string(),
        });

        anim_current_frame = (anim_current_frame + 1) % player.model_animations[2].frameCount;
        rl.update_model_animation(
            &thread,
            &mut player.model,
            &player.model_animations[2],
            anim_current_frame,
        );

        camera.position = player.position;
        camera.target = player.target;

        shader.set_shader_value(view_pos_loc, camera.position);

        //fetch textures.
        let players: Vec<(Vector3, f32, Option<HashMap<String, WeakTexture2D>>)> = net_client
            .remotePlayers
            .iter()
            .map(|p| {
                //Eh.. the urge to optimize things when its too late..
                // So check if we have the texture data for the current player
                let textures: Option<HashMap<String, WeakTexture2D>> =
                    sol_client.fetch_skin(&mut rl, &thread, &p.skin);
                (
                    Vector3 {
                        x: p.position_x,
                        y: p.position_y,
                        z: p.position_z,
                    },
                    p.yaw,
                    textures,
                )
            })
            .collect();

        // Draw
        {
            let mut dhl = rl.begin_drawing(&thread);
            dhl.clear_background(Color::WHITE);

            let mut d3d = dhl.begin_mode3D(&camera);

            // Draw the model at player position
            let (axis, angle) =
                (Quaternion::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), player.orientation.y)
                    * Quaternion::from_axis_angle(
                        Vector3::new(1.0, 0.0, 0.0),
                        player.orientation.x,
                    ))
                .to_axis_angle();

            d3d.draw_model_ex(
                &player.model,
                player.position,
                axis,
                angle.to_degrees(),
                Vector3::new(1.0, 1.0, 1.0),
                Color::WHITE,
            );

            d3d.draw_model(&map.model, Vector3::new(0.0, 0.0, 0.0), 1.0, Color::WHITE);

            // let world_box = BoundingBox {
            //     min: player.bounding_box.min + player.position,
            //     max: player.bounding_box.max + player.position,
            // };
            // draw_bounding_box(&mut d3d, &world_box, Color::RED);

            // // draw map bounding boxes for debugging
            // for bbox in &map.boundings {
            //     draw_bounding_box(&mut d3d, bbox, Color::GREEN);
            // }

            //Draw remote players
            players.iter().for_each(|(position, yaw, textures)| {
                //switch gun textures for each instance as we reuse the model.. normally one would use an atlas

                //the no texture material
                let mut material_count = 0;
                if let Some(textures) = textures {
                    //load textures
                    m_player.apply_gun_textures(textures);
                    material_count = 1;
                }

                m_player.draw(
                    &mut d3d,
                    &Matrix::translate(position.x, position.y, position.z),
                    &Matrix::rotate(Vector3::new(0.0, 1.0, 0.0), *yaw),
                    material_count,
                );

                // let world_box = BoundingBox {
                //     min: player.bounding_box.min
                //         + Vector3 {
                //             x: p.position_x,
                //             y: p.position_y,
                //             z: p.position_z,
                //         },
                //     max: player.bounding_box.max
                //         + Vector3 {
                //             x: p.position_x,
                //             y: p.position_y,
                //             z: p.position_z,
                //         },
                // };
                // draw_bounding_box(&mut d3d, &world_box, Color::RED);
            });

            drop(d3d);
            drop(dhl);
        }
    }

    //Unload textures
    unload_textures_from_model(&mut rl, &thread, &player.model);
    sol_client.clear(&mut rl, &thread);
}
