use rust_gpu_bridge::glam::{Vec3, Vec4};

use crate::prelude::{Lights, f_ab, env_brdf_approx};

// A precomputed `NdotV` is provided because it is computed regardless,
// but `world_normal` and the view vector `V` are provided separately for more advanced uses.
pub fn ambient_light<const MAX_DIRECTIONAL_LIGHTS: usize, const MAX_CASCADES_PER_LIGHT: usize>(
    lights: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
    world_position: Vec4,
    world_normal: Vec3,
    v: Vec3,
    n_dot_v: f32,
    diffuse_color: Vec3,
    specular_color: Vec3,
    perceptual_roughness: f32,
    occlusion: f32,
) -> Vec3 {
    let diffuse_ambient = env_brdf_approx(diffuse_color, f_ab(1.0, n_dot_v)) * occlusion;
    let specular_ambient = env_brdf_approx(specular_color, f_ab(perceptual_roughness, n_dot_v));

    return (diffuse_ambient + specular_ambient) * lights.ambient_color.truncate();
}
