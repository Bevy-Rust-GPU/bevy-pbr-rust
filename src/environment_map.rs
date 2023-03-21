use rust_gpu_bridge::{
    glam::{Vec2, Vec3},
    Pow,
};
use spirv_std::Sampler;

use crate::prelude::{Lights, TextureCube};

pub trait EnvironmentMap {
    fn environment_map_light<
        const MAX_DIRECTIONAL_LIGHTS: usize,
        const MAX_CASCADES_PER_LIGHT: usize,
    >(
        lights: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
        environment_map_diffuse: &TextureCube,
        environment_map_specular: &TextureCube,
        environment_map_sampler: &Sampler,
        perceptual_roughness: f32,
        roughness: f32,
        diffuse_color: Vec3,
        n_dot_v: f32,
        f_ab: Vec2,
        n: Vec3,
        r: Vec3,
        f0: Vec3,
    ) -> EnvironmentMapLight;
}

impl EnvironmentMap for () {
    fn environment_map_light<
        const MAX_DIRECTIONAL_LIGHTS: usize,
        const MAX_CASCADES_PER_LIGHT: usize,
    >(
        _: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
        _: &TextureCube,
        _: &TextureCube,
        _: &Sampler,
        _: f32,
        _: f32,
        _: Vec3,
        _: f32,
        _: Vec2,
        _: Vec3,
        _: Vec3,
        _: Vec3,
    ) -> EnvironmentMapLight {
        Default::default()
    }
}

#[derive(Default, Copy, Clone, PartialEq)]
pub struct EnvironmentMapLight {
    pub diffuse: Vec3,
    pub specular: Vec3,
}

impl EnvironmentMap for EnvironmentMapLight {
    fn environment_map_light<
        const MAX_DIRECTIONAL_LIGHTS: usize,
        const MAX_CASCADES_PER_LIGHT: usize,
    >(
        lights: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
        environment_map_diffuse: &TextureCube,
        environment_map_specular: &TextureCube,
        environment_map_sampler: &Sampler,
        perceptual_roughness: f32,
        roughness: f32,
        diffuse_color: Vec3,
        n_dot_v: f32,
        f_ab: Vec2,
        n: Vec3,
        r: Vec3,
        f0: Vec3,
    ) -> EnvironmentMapLight {
        // Split-sum approximation for image based lighting: https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
        // Technically we could use textureNumLevels(environment_map_specular) - 1 here, but we use a uniform
        // because textureNumLevels() does not work on WebGL2
        let radiance_level =
            perceptual_roughness * (lights.environment_map_smallest_specular_mip_level) as f32;
        let irradiance = environment_map_diffuse
            .sample::<f32>(*environment_map_sampler, n)
            .truncate();
        let radiance = environment_map_specular
            .sample_by_lod::<f32>(*environment_map_sampler, r, radiance_level)
            .truncate();

        // Multiscattering approximation: https://www.jcgt.org/published/0008/01/03/paper.pdf
        // Useful reference: https://bruop.github.io/ibl
        let fr = Vec3::splat(1.0 - roughness).max(f0) - f0;
        let ks = f0 + fr * (1.0 - n_dot_v).pow(5.0);
        let fss_ess = ks * f_ab.x + f_ab.y;
        let ess = f_ab.x + f_ab.y;
        let ems = 1.0 - ess;
        let favg = f0 + (1.0 - f0) / 21.0;
        let fms = fss_ess * favg / (1.0 - ems * favg);
        let fms_ems = fms * ems;
        let edss = 1.0 - (fss_ess + fms_ems);
        let kd = diffuse_color * edss;

        EnvironmentMapLight {
            diffuse: (fms_ems + kd) * irradiance,
            specular: fss_ess * radiance,
        }
    }
}
