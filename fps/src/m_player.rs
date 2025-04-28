use std::f32::consts::PI;

use raylib::{
    math::{Matrix, Quaternion, Vector3, Vector4}, models::{Model, ModelAnimation, RaylibModel, RaylibModelAnimation}, prelude::{RaylibDraw3D, RaylibDrawHandle, RaylibMode3D}, shaders::Shader, RaylibHandle, RaylibThread
};

use crate::utils::c_bytesto_string;


//The model of the 3rd person
pub struct MPlayer {
    pub model: Model,
    pub animations: Vec<ModelAnimation>,
    right_hand_bone: usize,
    pub ak_only: Model,
}

impl MPlayer {
    pub fn load(
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        shader: &Shader,
    ) -> Result<Self, String> {
        let mut m_player = rl.load_model(&thread, "resources/m_player.gltf").unwrap();
        let mut m_player_animations = rl
            .load_model_animations(&thread, "resources/m_player.gltf")
            .unwrap();

        // Find the head bone index
        let head_bone_index = m_player
            .bones()
            .unwrap()
            .iter()
            .position(|bone| c_bytesto_string(&bone.name).eq("mixamorig:Head"))
            .unwrap();

        let frame_poses = m_player_animations[0].frame_poses();
        let head_transform = frame_poses[0][head_bone_index];

        // Apply rotation and scale to the model
        let scale = Matrix::scale(0.01, 0.01, 0.01);
        let rotate = Matrix::rotate(Vector3::new(1.0, 0.0, 0.0), PI / 2.0);
        let translate = Matrix::translate(
            -head_transform.translation.x,
            -head_transform.translation.y,
            -head_transform.translation.z,
        );
        let transform = translate * rotate * scale;
        m_player.set_transform(&transform);
        rl.update_model_animation(&thread, &mut m_player, &m_player_animations[0], 0);

        // Apply shader to model
        for i in 0..m_player.materials().len() {
            let material = &mut m_player.materials_mut()[i];
            material.shader = (*shader).clone();
        }

        let right_hand_bone = m_player
            .bones()
            .unwrap()
            .iter()
            .position(|bone| c_bytesto_string(&bone.name).eq("mixamorig:RightHand"))
            .unwrap();

        let ak_only = rl.load_model(&thread, "resources/ak_only.glb").unwrap();

        Ok(MPlayer {
            model: m_player,
            animations: m_player_animations,
            right_hand_bone,
            ak_only
        })
    }

    pub fn draw(&self, d3d: &mut RaylibMode3D<RaylibDrawHandle>, translate: &Matrix, rotate: &Matrix)
    {
        let model_transform = self.model.transform().clone() * (*rotate) * (*translate);

        for mesh in self.model.meshes() {
            d3d.draw_mesh(mesh.clone(), self.model.materials()[1].clone(), model_transform);
        }

        //Put gun in hands
        let hand_transform = self.animations[0].frame_poses()[0][self.right_hand_bone];

        //Welp weak support in this wrapper so..
        let in_rotation =
            unsafe { *self.model.bindPose.wrapping_add(self.right_hand_bone) }.rotation;
        let in_rotation = Vector4 {
            x: in_rotation.x,
            y: in_rotation.y,
            z: in_rotation.z,
            w: in_rotation.w,
        };
        let out_rotation = hand_transform.rotation;
        let corrected_rotation = out_rotation * in_rotation.inverted();

        let mut gun_transform = corrected_rotation.to_matrix();
        gun_transform = gun_transform
            * Matrix::translate(
                hand_transform.translation.x,
                hand_transform.translation.y,
                hand_transform.translation.z,
            );

        gun_transform = gun_transform * model_transform;

        let gun_scale = Matrix::scale(86.0, 86.0, 86.0);
        let gun_rotate = (Quaternion::from_axis_angle(
            Vector3::new(0.0, 0.0, 1.0),
            -103.0_f32.to_radians(),
        ) * Quaternion::from_axis_angle(
            Vector3::new(1.0, 0.0, 0.0),
            -162.0_f32.to_radians(),
        ) * Quaternion::from_axis_angle(
            Vector3::new(0.0, 1.0, 0.0),
            2.0_f32.to_radians(),
        ))
        .to_matrix();

        let gun_offset = Matrix::translate(0.0, 0.0, 0.08);
        let combined_offset = gun_offset * gun_rotate * gun_scale;

        //The whole transform is Local_T * Rot_A * Translate_A * Model_T
        gun_transform = combined_offset * gun_transform;

        d3d.draw_mesh(
            self.ak_only.meshes()[0].clone(),
            self.ak_only.materials()[0].clone(),
            gun_transform,
        );
    }

}
