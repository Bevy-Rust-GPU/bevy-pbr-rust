use core::ops::{Add, Mul, Sub};

use spirv_std::{
    arch::unsigned_min,
    glam::{UVec4, Vec2, Vec3, Vec4},
    Sampler,
};

#[allow(unused_imports)]
use spirv_std::num_traits::Float;

use rust_gpu_bridge::{hsv2rgb, mix::Mix, prelude::NaturalLog};

use crate::prelude::{DirectionalLight, DirectionalShadowTextures, View};

#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Lights<const MAX_DIRECTIONAL_LIGHTS: usize, const MAX_CASCADES_PER_LIGHT: usize> {
    // NOTE: this array size must be kept in sync with the constants defined in bevy_pbr/src/render/light.rs
    pub directional_lights: [DirectionalLight<MAX_CASCADES_PER_LIGHT>; MAX_DIRECTIONAL_LIGHTS],
    pub ambient_color: Vec4,
    // x/y/z dimensions and n_clusters in w
    pub cluster_dimensions: UVec4,
    // xy are Vec2(cluster_dimensions.xy) / Vec2(view.width, view.height)
    //
    // For perspective projections:
    // z is cluster_dimensions.z / log(far / near)
    // w is cluster_dimensions.z * log(near) / log(far / near)
    //
    // For orthographic projections:
    // NOTE: near and far are +ve but -z is infront of the camera
    // z is -near
    // w is cluster_dimensions.z / (-far - -near)
    pub cluster_factors: Vec4,
    pub n_directional_lights: u32,
    pub spot_light_shadowmap_offset: i32,
    pub environment_map_smallest_specular_mip_level: u32,
}

impl<const MAX_DIRECTIONAL_LIGHTS: usize, const MAX_CASCADES_PER_LIGHT: usize> Default
    for Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>
{
    fn default() -> Self {
        Lights {
            directional_lights: [Default::default(); MAX_DIRECTIONAL_LIGHTS],
            ambient_color: Default::default(),
            cluster_dimensions: Default::default(),
            cluster_factors: Default::default(),
            n_directional_lights: Default::default(),
            spot_light_shadowmap_offset: Default::default(),
            environment_map_smallest_specular_mip_level: Default::default(),
        }
    }
}

impl<const MAX_DIRECTIONAL_LIGHTS: usize, const MAX_CASCADES_PER_LIGHT: usize>
    Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>
{
    pub fn get_cascade_index(&self, light_id: u32, view_z: f32) -> u32 {
        let light = &self.directional_lights[light_id as usize];

        for i in 0..light.num_cascades {
            if -view_z < light.cascades[i as usize].far_bound {
                return i;
            }
        }
        return (*light).num_cascades;
    }

    pub fn sample_cascade<DS: DirectionalShadowTextures>(
        &self,
        directional_shadow_textures: &DS,
        directional_shadow_textures_sampler: &Sampler,
        light_id: u32,
        cascade_index: u32,
        frag_position: Vec4,
        surface_normal: Vec3,
    ) -> f32 {
        let light = &self.directional_lights[light_id as usize];
        let cascade = &(*light).cascades[cascade_index as usize];
        // The normal bias is scaled to the texel size.
        let normal_offset = (*light).shadow_normal_bias * cascade.texel_size * surface_normal;
        let depth_offset = (*light).shadow_depth_bias * light.direction_to_light;
        let offset_position =
            (frag_position.truncate() + normal_offset + depth_offset).extend(frag_position.w);

        let offset_position_clip = cascade.view_projection * offset_position;
        if offset_position_clip.w <= 0.0 {
            return 1.0;
        }
        let offset_position_ndc = offset_position_clip.truncate() / offset_position_clip.w;
        // No shadow outside the orthographic projection volume
        if (offset_position_ndc.x < -1.0 || offset_position_ndc.y < -1.0)
            || offset_position_ndc.z < 0.0
            || (offset_position_ndc.x > 1.0
                || offset_position_ndc.y > 1.0
                || offset_position_ndc.z > 1.0)
        {
            return 1.0;
        }

        // compute texture coordinates for shadow lookup, compensating for the Y-flip difference
        // between the NDC and texture coordinates
        let flip_correction = Vec2::new(0.5, -0.5);
        let light_local = offset_position_ndc.truncate() * flip_correction + Vec2::new(0.5, 0.5);

        let depth = offset_position_ndc.z;
        // do the lookup, using HW PCF and comparison
        // NOTE: Due to non-uniform control flow above, we must use the level variant of the texture
        // sampler to avoid use of implicit derivatives causing possible undefined behavior.
        directional_shadow_textures.sample_depth_reference(
            directional_shadow_textures_sampler,
            light_local,
            depth,
            light.depth_texture_base_index.add(cascade_index),
            0,
        )
    }

    pub fn fetch_directional_shadow<DS: DirectionalShadowTextures>(
        &self,
        directional_shadow_textures: &DS,
        directional_shadow_textures_sampler: &Sampler,
        light_id: u32,
        frag_position: Vec4,
        surface_normal: Vec3,
        view_z: f32,
    ) -> f32 {
        let light = &self.directional_lights[light_id as usize];
        let cascade_index = self.get_cascade_index(light_id, view_z);

        if cascade_index >= (*light).num_cascades {
            return 1.0;
        }

        let mut shadow = self.sample_cascade(
            directional_shadow_textures,
            directional_shadow_textures_sampler,
            light_id,
            cascade_index,
            frag_position,
            surface_normal,
        );

        // Blend with the next cascade, if there is one.
        let next_cascade_index = cascade_index.add(1);
        if next_cascade_index < (*light).num_cascades {
            let this_far_bound = (*light).cascades[cascade_index as usize].far_bound;
            let next_near_bound = (1.0 - (*light).cascades_overlap_proportion) * this_far_bound;
            if -view_z >= next_near_bound {
                let next_shadow = self.sample_cascade(
                    directional_shadow_textures,
                    directional_shadow_textures_sampler,
                    light_id,
                    next_cascade_index,
                    frag_position,
                    surface_normal,
                );
                shadow = shadow.mix(
                    next_shadow,
                    (-view_z - next_near_bound) / (this_far_bound - next_near_bound),
                );
            }
        }
        return shadow;
    }

    pub fn cascade_debug_visualization(&self, output_color: Vec3, light_id: u32, view_z: f32) -> Vec3 {
        let overlay_alpha = 0.95;
        let cascade_index = self.get_cascade_index(light_id, view_z);
        let cascade_color = hsv2rgb(
            cascade_index as f32 / (MAX_CASCADES_PER_LIGHT + 1) as f32,
            1.0,
            0.5,
        );

        (1.0 - overlay_alpha) * output_color + overlay_alpha * cascade_color
    }

    // NOTE: Keep in sync with bevy_pbr/src/light.rs
    pub fn view_z_to_z_slice(&self, view_z: f32, is_orthographic: bool) -> u32 {
        let z_slice = if is_orthographic {
            // NOTE: view_z is correct in the orthographic case
            ((view_z - self.cluster_factors.z) * self.cluster_factors.w).floor() as u32
        } else {
            // NOTE: had to use -view_z to make it positive else log(negative) is nan
            ((-view_z).natural_log() * self.cluster_factors.z - self.cluster_factors.w + 1.0) as u32
        };
        // NOTE: We use min as we may limit the far z plane used for clustering to be closer than
        // the furthest thing being drawn. This means that we need to limit to the maximum cluster.
        unsigned_min(z_slice, self.cluster_dimensions.z.sub(1))
    }

    pub fn fragment_cluster_index(
        &self,
        view: &View,
        frag_coord: Vec2,
        view_z: f32,
        is_orthographic: bool,
    ) -> u32 {
        let xy = ((frag_coord - view.viewport.truncate().truncate())
            * self.cluster_factors.truncate().truncate())
        .floor()
        .as_uvec2();
        let z_slice = self.view_z_to_z_slice(view_z, is_orthographic);
        // NOTE: Restricting cluster index to avoid undefined behavior when accessing uniform buffer
        // arrays based on the cluster index.
        unsigned_min(
            (xy.y.mul(self.cluster_dimensions.x).add(xy.x))
                .mul(self.cluster_dimensions.z)
                .add(z_slice),
            self.cluster_dimensions.w.sub(1),
        )
    }
}
