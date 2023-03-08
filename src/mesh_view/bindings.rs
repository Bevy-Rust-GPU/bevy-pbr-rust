use spirv_std::{spirv, Image, Sampler};

use crate::{
    fog::Fog,
    prelude::{
        ClusterLightIndexLists, ClusterOffsetsAndCounts, DirectionalShadowTextures, Globals,
        Lights, PointLights, PointShadowTextures, View,
    },
};

pub type Texture2d = Image!(2D, type = f32, sampled = true);
pub type Texture3d = Image!(3D, type = f32, sampled = true);
pub type TextureCube = Image!(cube, type = f32, sampled = true);
pub type TextureDepthCube = Image!(cube, type = f32, sampled = true, depth = true);
pub type TextureDepthCubeArray =
    Image!(cube, type = f32, sampled = true, depth = true, arrayed = true);

pub type TextureDepth2d = Image!(2D, type = f32, sampled = true, depth = true);
pub type TextureDepth2dArray = Image!(2D, type = f32, sampled = true, depth = true, arrayed = true);

pub type TextureMultisampled2d = Image!(2D, type = f32, sampled = true, multisampled = true);
pub type TextureDepthMultisampled2d =
    Image!(2D, type = f32, sampled = true, multisampled = true, depth = true);

pub trait DepthPrepassTexture {}

impl DepthPrepassTexture for TextureDepthMultisampled2d {}
impl DepthPrepassTexture for TextureDepth2d {}

pub trait NormalPrepassTexture {}

impl NormalPrepassTexture for TextureMultisampled2d {}
impl NormalPrepassTexture for Texture2d {}

#[allow(unused_variables)]
#[spirv(fragment)]
pub fn mesh_view_bindings<
    const MAX_DIRECTIONAL_LIGHTS: usize,
    const MAX_CASCADES_PER_LIGHT: usize,
>(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] view: &View,
    #[spirv(descriptor_set = 0, binding = 1)] lights: &Lights<
        MAX_DIRECTIONAL_LIGHTS,
        MAX_CASCADES_PER_LIGHT,
    >,
    #[spirv(descriptor_set = 0, binding = 2)] point_shadow_textures: &impl PointShadowTextures,
    #[spirv(descriptor_set = 0, binding = 3)] point_shadow_textures_sampler: &Sampler,
    #[spirv(descriptor_set = 0, binding = 4)]
    directional_shadow_textures: &impl DirectionalShadowTextures,
    #[spirv(descriptor_set = 0, binding = 5)] directional_shadow_textures_sampler: &Sampler,
    #[spirv(descriptor_set = 0, binding = 6)] point_lights: &impl PointLights,
    #[spirv(descriptor_set = 0, binding = 7)] cluster_light_index_lists: &impl ClusterLightIndexLists,
    #[spirv(descriptor_set = 0, binding = 8)] cluster_offsets_and_counts: &impl ClusterOffsetsAndCounts,
    #[spirv(descriptor_set = 0, binding = 9)] globals: &Globals,
    #[spirv(descriptor_set = 0, binding = 10)] fog: &Fog,
    #[spirv(descriptor_set = 0, binding = 11)] environment_map_diffuse: &TextureCube,
    #[spirv(descriptor_set = 0, binding = 12)] environment_map_specular: &TextureCube,
    #[spirv(descriptor_set = 0, binding = 13)] environment_map_sampler: &Sampler,
    #[spirv(descriptor_set = 0, binding = 14)] dt_lut_texture: &Texture3d,
    #[spirv(descriptor_set = 0, binding = 15)] dt_lut_sampler: &Sampler,
    #[spirv(descriptor_set = 0, binding = 16)] depth_prepass_texture: &impl DepthPrepassTexture,
    #[spirv(descriptor_set = 0, binding = 17)] normal_prepass_texture: &impl NormalPrepassTexture,
) {
}
