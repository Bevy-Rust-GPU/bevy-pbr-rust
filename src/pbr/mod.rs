pub mod ambient;
pub mod bindings;
pub mod entry_points;
pub mod lighting;
pub mod standard_material;

use core::ops::Add;

use spirv_std::{
    glam::{Vec3, Vec4},
    Sampler,
};

use rust_gpu_bridge::{glam::Vec2, hsv2rgb, Pow, Reflect};

use crate::{
    environment_map::EnvironmentMapLight,
    fog::{
        Fog, FOG_MODE_ATMOSPHERIC, FOG_MODE_EXPONENTIAL, FOG_MODE_EXPONENTIAL_SQUARED,
        FOG_MODE_LINEAR,
    },
    prelude::{
        perceptual_roughness_to_roughness, screen_space_dither, ClusterDebugVisualization,
        ClusterLightIndexLists, ClusterOffsetsAndCounts, DirectionalShadowTextures, Lights, Mesh,
        PointLights, PointShadowTextures, StandardMaterial, TextureCube, View,
        DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT, MESH_FLAGS_SHADOW_RECEIVER_BIT,
        POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT,
    },
};

use self::{
    ambient::ambient_light,
    lighting::f_ab,
    standard_material::{
        STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS,
    },
};

#[repr(C)]
pub struct BaseMaterial {
    pub base: StandardMaterial,
}

#[repr(C)]
pub struct PbrInput {
    pub material: StandardMaterial,
    pub occlusion: f32,
    pub frag_coord: Vec4,
    pub world_position: Vec4,
    // Normalized world normal used for shadow mapping as normal-mapping is not used for shadow
    // mapping
    pub world_normal: Vec3,
    // Normalized normal-mapped world normal used for lighting
    pub n: Vec3,
    // Normalized view vector in world space, pointing from the fragment world position toward the
    // view world position
    pub v: Vec3,
    pub is_orthographic: bool,
    pub flags: u32,
}

impl Default for PbrInput {
    fn default() -> Self {
        PbrInput {
            material: StandardMaterial::default(),
            occlusion: 1.0,

            frag_coord: Vec4::new(0.0, 0.0, 0.0, 1.0),
            world_position: Vec4::new(0.0, 0.0, 0.0, 1.0),
            world_normal: Vec3::new(0.0, 0.0, 1.0),

            is_orthographic: false,

            n: Vec3::new(0.0, 0.0, 1.0),
            v: Vec3::new(1.0, 0.0, 0.0),

            flags: 0,
        }
    }
}

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

impl PbrInput {
    pub fn pbr<
        const MAX_DIRECTIONAL_LIGHTS: usize,
        const MAX_CASCADES_PER_LIGHT: usize,
        PL: PointLights,
        DS: DirectionalShadowTextures,
        PS: PointShadowTextures,
        CL: ClusterLightIndexLists,
        CO: ClusterOffsetsAndCounts,
        EM: EnvironmentMap,
        CD: ClusterDebugVisualization,
        DD: DirectionalLightShadowMapDebug,
    >(
        &self,
        view: &View,
        _: &Mesh,
        lights: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
        point_lights: &PL,
        cluster_light_index_lists: &CL,
        cluster_offsets_and_counts: &CO,
        directional_shadow_textures: &DS,
        directional_shadow_textures_sampler: &Sampler,
        point_shadow_textures: &PS,
        point_shadow_textures_sampler: &Sampler,
        environment_map_diffuse: &TextureCube,
        environment_map_specular: &TextureCube,
        environment_map_sampler: &Sampler,
    ) -> Vec4 {
        let mut output_color = self.material.base_color;

        // TODO use .a for exposure compensation in HDR
        let emissive = self.material.emissive;

        // calculate non-linear roughness from linear perceptualRoughness
        let metallic = self.material.metallic;
        let perceptual_roughness = self.material.perceptual_roughness;
        let roughness = perceptual_roughness_to_roughness(perceptual_roughness);

        let occlusion = self.occlusion;

        output_color = self.material.alpha_discard(output_color);

        // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
        let n_dot_v = self.n.dot(self.v).max(0.0001);

        // Remapping [0,1] reflectance to F0
        // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
        let reflectance = self.material.reflectance;
        let f0 = 0.16 * reflectance * reflectance * (1.0 - metallic)
            + output_color.truncate() * metallic;

        // Diffuse strength inversely related to metallicity
        let diffuse_color = output_color.truncate() * (1.0 - metallic);

        let r = -self.v.reflect(self.n);

        let f_ab = f_ab(perceptual_roughness, n_dot_v);

        let mut direct_light = Vec3::ZERO;

        let view_z = Vec4::new(
            view.inverse_view.x_axis.z,
            view.inverse_view.y_axis.z,
            view.inverse_view.z_axis.z,
            view.inverse_view.w_axis.z,
        )
        .dot(self.world_position);
        let cluster_index = lights.fragment_cluster_index(
            view,
            self.frag_coord.truncate().truncate(),
            view_z,
            self.is_orthographic,
        );
        let offset_and_counts = cluster_offsets_and_counts.unpack(cluster_index);

        // Point lights (direct)
        for i in offset_and_counts.x as u32..(offset_and_counts.x.add(offset_and_counts.y)) as u32 {
            let light_id = cluster_light_index_lists.get_light_id(i);

            let mut shadow: f32 = 1.0;
            if (self.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0
                && (point_lights.get_point_light(light_id).flags
                    & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT)
                    != 0
            {
                shadow = point_lights.fetch_point_shadow(
                    point_shadow_textures,
                    point_shadow_textures_sampler,
                    light_id,
                    self.world_position,
                    self.world_normal,
                );
            }

            let light_contrib = point_lights.get_point_light(light_id).point_light(
                self.world_position.truncate(),
                light_id,
                roughness,
                n_dot_v,
                self.n,
                self.v,
                r,
                f0,
                f_ab,
                diffuse_color,
            );
            direct_light += light_contrib * shadow;
        }

        // Spot lights (direct)
        for i in (offset_and_counts.x.add(offset_and_counts.y)) as u32
            ..(offset_and_counts
                .x
                .add(offset_and_counts.y)
                .add(offset_and_counts.z)) as u32
        {
            let light_id = cluster_light_index_lists.get_light_id(i);
            let light = point_lights.get_point_light(light_id);

            let mut shadow: f32 = 1.0;
            if (self.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0
                && (light.flags & POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0
            {
                shadow = point_lights.fetch_spot_shadow(
                    lights,
                    directional_shadow_textures,
                    directional_shadow_textures_sampler,
                    light_id,
                    self.world_position,
                    self.world_normal,
                );
            }

            let light_contrib = light.spot_light(
                self.world_position.truncate(),
                light_id,
                roughness,
                n_dot_v,
                self.n,
                self.v,
                r,
                f0,
                f_ab,
                diffuse_color,
            );
            direct_light += light_contrib * shadow;
        }

        // Directional lights (direct)
        let n_directional_lights = lights.n_directional_lights;
        for i in 0..n_directional_lights {
            let directional_light = &lights.directional_lights[i as usize];

            let mut shadow: f32 = 1.0;
            if (self.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0
                && (lights.directional_lights[i as usize].flags
                    & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT)
                    != 0
            {
                shadow = lights.fetch_directional_shadow(
                    directional_shadow_textures,
                    directional_shadow_textures_sampler,
                    i,
                    self.world_position,
                    self.world_normal,
                    view_z,
                );
            }

            let mut light_contrib = directional_light.directional_light(
                i,
                roughness,
                n_dot_v,
                self.n,
                self.v,
                r,
                f0,
                f_ab,
                diffuse_color,
            );

            light_contrib = DD::cascade_debug_visualization(lights, light_contrib, i, view_z);

            direct_light += light_contrib * shadow;
        }

        // Ambient light (indirect)
        let mut indirect_light = ambient_light(
            lights,
            self.world_position,
            self.n,
            self.v,
            n_dot_v,
            diffuse_color,
            f0,
            perceptual_roughness,
            occlusion,
        );

        // Environment map light (indirect)
        //#ifdef ENVIRONMENT_MAP
        let environment_light = EM::environment_map_light(
            lights,
            environment_map_diffuse,
            environment_map_specular,
            environment_map_sampler,
            perceptual_roughness,
            roughness,
            diffuse_color,
            n_dot_v,
            f_ab,
            self.n,
            r,
            f0,
        );

        indirect_light += (environment_light.diffuse * occlusion) + environment_light.specular;
        //#endif

        let emissive_light = emissive.truncate() * output_color.w;

        // Total light
        output_color = (direct_light + indirect_light + emissive_light).extend(output_color.w);

        output_color = CD::cluster_debug_visualization(
            lights,
            output_color,
            view_z,
            self.is_orthographic,
            offset_and_counts,
            cluster_index,
        );

        output_color
    }
}

// Cluster allocation debug (using 'over' alpha blending)
pub trait DirectionalLightShadowMapDebug {
    fn cascade_debug_visualization<
        const MAX_DIRECTIONAL_LIGHTS: usize,
        const MAX_CASCADES_PER_LIGHT: usize,
    >(
        lights: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
        output_color: Vec3,
        light_id: u32,
        view_z: f32,
    ) -> Vec3;
}

impl DirectionalLightShadowMapDebug for () {
    fn cascade_debug_visualization<
        const MAX_DIRECTIONAL_LIGHTS: usize,
        const MAX_CASCADES_PER_LIGHT: usize,
    >(
        _: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
        output_color: Vec3,
        _: u32,
        _: f32,
    ) -> Vec3 {
        output_color
    }
}

pub enum DebugCascades {}

impl DirectionalLightShadowMapDebug for DebugCascades {
    fn cascade_debug_visualization<
        const MAX_DIRECTIONAL_LIGHTS: usize,
        const MAX_CASCADES_PER_LIGHT: usize,
    >(
        lights: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
        output_color: Vec3,
        light_id: u32,
        view_z: f32,
    ) -> Vec3 {
        let overlay_alpha = 0.95;
        let cascade_index = lights.get_cascade_index(light_id, view_z);
        let cascade_color = hsv2rgb(
            cascade_index as f32 / (MAX_CASCADES_PER_LIGHT.add(1)) as f32,
            1.0,
            0.5,
        );

        (1.0 - overlay_alpha) * output_color + overlay_alpha * cascade_color
    }
}

pub fn dither(color: Vec4, pos: Vec2) -> Vec4 {
    return (color.truncate() + screen_space_dither(pos)).extend(color.w);
}

pub fn apply_fog<const MAX_DIRECTIONAL_LIGHTS: usize, const MAX_CASCADES_PER_LIGHT: usize>(
    fog: &Fog,
    lights: &Lights<MAX_DIRECTIONAL_LIGHTS, MAX_CASCADES_PER_LIGHT>,
    input_color: Vec4,
    fragment_world_position: Vec3,
    view_world_position: Vec3,
) -> Vec4 {
    let view_to_world = fragment_world_position - view_world_position;

    // `length()` is used here instead of just `view_to_world.z` since that produces more
    // high quality results, especially for denser/smaller fogs. we get a "curved"
    // fog shape that remains consistent with camera rotation, instead of a "linear"
    // fog shape that looks a bit fake
    let distance = view_to_world.length();

    let mut scattering = Vec3::ZERO;
    if fog.directional_light_color.w > 0.0 {
        let view_to_world_normalized = view_to_world / distance;
        let n_directional_lights = lights.n_directional_lights;
        for i in 0..n_directional_lights {
            let light = lights.directional_lights[i as usize];
            scattering += (view_to_world_normalized
                .dot(light.direction_to_light)
                .max(0.0))
            .pow(fog.directional_light_exponent)
                * light.color.truncate();
        }
    }

    if fog.mode == FOG_MODE_LINEAR {
        return fog.linear_fog(input_color, distance, scattering);
    } else if fog.mode == FOG_MODE_EXPONENTIAL {
        return fog.exponential_fog(input_color, distance, scattering);
    } else if fog.mode == FOG_MODE_EXPONENTIAL_SQUARED {
        return fog.exponential_squared_fog(input_color, distance, scattering);
    } else if fog.mode == FOG_MODE_ATMOSPHERIC {
        return fog.atmospheric_fog(input_color, distance, scattering);
    } else {
        return input_color;
    }
}

pub trait PremultiplyAlpha {
    fn premultiply_alpha(standard_material_flags: u32, color: Vec4) -> Vec4;
}

impl PremultiplyAlpha for () {
    fn premultiply_alpha(_: u32, color: Vec4) -> Vec4 {
        color
    }
}

pub enum Multiply {}

impl PremultiplyAlpha for Multiply {
    fn premultiply_alpha(_: u32, color: Vec4) -> Vec4 {
        // `Multiply` uses its own `BlendState`, but we still need to premultiply here in the
        // shader so that we get correct results as we tweak the alpha channel

        // The blend function is:
        //
        //     result = dst_color * src_color + (1 - src_alpha) * dst_color
        //
        // We premultiply `src_color` by `src_alpha`:
        //
        //     src_color *= src_alpha
        //
        // We end up with:
        //
        //     result = dst_color * (src_color * src_alpha) + (1 - src_alpha) * dst_color
        //     result = src_alpha * (src_color * dst_color) + (1 - src_alpha) * dst_color
        //
        // Which is the blend operation for multiplicative blending with arbitrary mixing
        // controlled by the source alpha channel
        return (color.truncate() * color.w).extend(color.w);
    }
}

pub enum BlendPremultipliedAlpha {}

impl PremultiplyAlpha for BlendPremultipliedAlpha {
    fn premultiply_alpha(standard_material_flags: u32, color: Vec4) -> Vec4 {
        // `Blend`, `Premultiplied` and `Alpha` all share the same `BlendState`. Depending
        // on the alpha mode, we premultiply the color channels by the alpha channel value,
        // (and also optionally replace the alpha value with 0.0) so that the result produces
        // the desired blend mode when sent to the blending operation.

        // For `BlendState::PREMULTIPLIED_ALPHA_BLENDING` the blend function is:
        //
        //     result = 1 * src_color + (1 - src_alpha) * dst_color
        let alpha_mode = standard_material_flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
        if alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD {
            // Here, we premultiply `src_color` by `src_alpha`, and replace `src_alpha` with 0.0:
            //
            //     src_color *= src_alpha
            //     src_alpha = 0.0
            //
            // We end up with:
            //
            //     result = 1 * (src_alpha * src_color) + (1 - 0) * dst_color
            //     result = src_alpha * src_color + 1 * dst_color
            //
            // Which is the blend operation for additive blending
            return (color.truncate() * color.w).extend(0.0);
        } else {
            // Here, we don't do anything, so that we get premultiplied alpha blending. (As expected)
            return color;
        }
    }
}
