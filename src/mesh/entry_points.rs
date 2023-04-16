use spirv_std::{
    glam::{Vec2, Vec3, Vec4},
    spirv,
};

use permutate_macro::permutate;

use crate::{
    prelude::{Mesh, View},
    skinning::skin_normals,
};

use super::mesh_position_local_to_world;

#[spirv(vertex)]
#[allow(non_snake_case)]
#[permutate(
    parameters = {
        tangent: some | none,
        color: some | none,
        skinned: some | none
    },
    constants = {},
    types = {},
    permutations = [
        {
            parameters = [
                none,
                none,
                none
            ],
            constants = {},
            types = {}
        },
        {
            parameters = [
                some,
                some,
                some
            ],
            constants = {},
            types = {}
        },
        file("../../entry_points.json", "mesh::entry_points"),
        env("BEVY_PBR_RUST_MESH_VERTEX_PERMUTATIONS", "mesh::entry_points")
    ]
)]
pub fn vertex(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] view: &View,
    #[spirv(uniform, descriptor_set = 2, binding = 0)] mesh: &Mesh,

    #[permutate(skinned = some)]
    #[spirv(uniform, descriptor_set = 2, binding = 1)]
    joint_matrices: &crate::prelude::SkinnedMesh,

    in_position: Vec3,
    in_normal: Vec3,
    in_uv: Vec2,

    #[permutate(tangent = some)] in_tangent: Vec4,

    #[permutate(color = some)] in_color: Vec4,

    #[permutate(skinned = some)] in_joint_indices: rust_gpu_bridge::glam::UVec4,
    #[permutate(skinned = some)] in_joint_weights: rust_gpu_bridge::glam::Vec4,

    #[spirv(position)] out_clip_position: &mut Vec4,
    out_world_position: &mut Vec4,
    out_world_normal: &mut Vec3,
    out_uv: &mut Vec2,
    #[permutate(tangent = some)] out_tangent: &mut Vec4,
    #[permutate(color = some)] out_color: &mut Vec4,
) {
    let mut in_position = in_position.extend(1.0);
    let mut in_normal = in_normal;

    #[permutate(tangent = some)]
    let mut in_tangent = in_tangent;

    #[permutate(skinned = some)]
    let model = in_joint_weights.x * joint_matrices.data[in_joint_indices.x as usize]
        + in_joint_weights.y * joint_matrices.data[in_joint_indices.y as usize]
        + in_joint_weights.z * joint_matrices.data[in_joint_indices.z as usize]
        + in_joint_weights.w * joint_matrices.data[in_joint_indices.w as usize];

    #[permutate(skinned = none)]
    let model = mesh.model;

    #[permutate(skinned = some)]
    in_normal = skin_normals(model, in_normal);
    #[permutate(skinned = none)]
    in_normal = mesh.mesh_normal_local_to_world(in_normal);

    in_position = mesh_position_local_to_world(model, in_position);
    *out_clip_position = view.mesh_position_world_to_clip(in_position);

    #[permutate(tangent = some)]
    in_tangent = mesh.mesh_tangent_local_to_world(model, in_tangent);

    *out_world_position = in_position;
    *out_world_normal = in_normal;
    *out_uv = in_uv;

    #[permutate(tangent = some)]
    *out_tangent = in_tangent;

    #[permutate(color = some)]
    *out_color = in_color;
}

#[spirv(fragment)]
#[allow(unused_variables)]
pub fn fragment(in_world_position: Vec4, in_world_normal: Vec3, in_uv: Vec2, out_color: &mut Vec4) {
    *out_color = Vec4::new(1.0, 0.0, 1.0, 1.0);
}
