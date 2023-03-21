use spirv_std::glam::{Mat4, Vec3, Vec4};

use rust_gpu_bridge::{glam::Vec2, Saturate};

use crate::prelude::{fd_burley, specular};

pub const DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT: u32 = 1;

#[derive(Default, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct DirectionalCascade {
    pub view_projection: Mat4,
    pub texel_size: f32,
    pub far_bound: f32,
}

#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct DirectionalLight<const MAX_CASCADES_PER_LIGHT: usize> {
    pub cascades: [DirectionalCascade; MAX_CASCADES_PER_LIGHT],
    pub color: Vec4,
    pub direction_to_light: Vec3,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    pub flags: u32,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
    pub num_cascades: u32,
    pub cascades_overlap_proportion: f32,
    pub depth_texture_base_index: u32,
}

impl<const MAX_CASCADES_PER_LIGHT: usize> Default for DirectionalLight<MAX_CASCADES_PER_LIGHT> {
    fn default() -> Self {
        DirectionalLight {
            cascades: [Default::default(); MAX_CASCADES_PER_LIGHT],
            color: Default::default(),
            direction_to_light: Default::default(),
            flags: Default::default(),
            shadow_depth_bias: Default::default(),
            shadow_normal_bias: Default::default(),
            num_cascades: Default::default(),
            cascades_overlap_proportion: Default::default(),
            depth_texture_base_index: Default::default(),
        }
    }
}

impl<const MAX_CASCADES_PER_LIGHT: usize> DirectionalLight<MAX_CASCADES_PER_LIGHT> {
    pub fn directional_light(
        &self,
        _: u32,
        roughness: f32,
        n_dot_v: f32,
        normal: Vec3,
        view: Vec3,
        _: Vec3,
        f0: Vec3,
        f_ab: Vec2,
        diffuse_color: Vec3,
    ) -> Vec3 {
        let incident_light = self.direction_to_light;

        let half_vector = (incident_light + view).normalize();
        let nol = (normal.dot(incident_light)).saturate();
        let noh = (normal.dot(half_vector)).saturate();
        let loh = (incident_light.dot(half_vector)).saturate();

        let diffuse = diffuse_color * fd_burley(roughness, n_dot_v, nol, loh);
        let specular_intensity = 1.0;
        let specular_light = specular(
            f0,
            roughness,
            half_vector,
            n_dot_v,
            nol,
            noh,
            loh,
            specular_intensity,
            f_ab,
        );

        (specular_light + diffuse) * self.color.truncate() * nol
    }
}
