use permutate_macro::permutate;
use spirv_std::{
    glam::{Vec2, Vec3, Vec4},
    spirv, Sampler,
};

use crate::{
    fog::{Fog, FOG_MODE_OFF},
    pbr::{apply_fog, PremultiplyAlpha},
    prelude::{
        powsafe, BaseColorTexture, EmissiveTexture, Lights, Mesh, MetallicRoughnessTexture,
        NormalMapTexture, OcclusionTexture, PbrInput, TextureCube, View,
        STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT, STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT,
        STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT, STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y,
        STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT,
        STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT,
        STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT,
        STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP, STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
    },
};

use spirv_std::num_traits::Float;

use super::BaseMaterial;

#[permutate(
    parameters = {
        texture_format: texture | array,
        buffer_format: uniform | storage,
        uv: some | none,
        tangent: some | none,
        color: some | none,
        normal_map: some | none,
        skinned: some | none,
        tonemap: some | none,
        deband: some | none,
        blend_mode: multiply | blend_premultiplied_alpha | none,
        environment_map: some | none,
        premultiply_alpha: some | none,
        cluster_debug: debug_z_slices | debug_cluster_light_complexity | debug_cluster_coherency | none,
        directional_light_shadow_map_debug: some | none
    },
    constants = {
        MAX_DIRECTIONAL_LIGHTS: u32,
        MAX_CASCADES_PER_LIGHT: u32
    },
    permutations = [
        // All on
        {
            parameters = [
                array, uniform, some, some, some, some, some, some, some, multiply, some, some, debug_z_slices, some
            ],
            constants = {
                MAX_DIRECTIONAL_LIGHTS = 10,
                MAX_CASCADES_PER_LIGHT = 4
            }
        },
        // All off
        {
            parameters = [
                array, uniform, none, none, none, none, none, none, none, blend_premultiplied_alpha, none, none, none, none
            ],
            constants = {
                MAX_DIRECTIONAL_LIGHTS = 10,
                MAX_CASCADES_PER_LIGHT = 4
            }
        },
        file("../../entry_points.json", "pbr::entry_points"),
        env("BEVY_PBR_RUST_PBR_FRAGMENT_PERMUTATIONS", "pbr::entry_points")
    ]
)]
#[spirv(fragment)]
#[allow(non_snake_case)]
pub fn fragment(
    #[spirv(uniform, descriptor_set = 0, binding = 0)] view: &View,
    #[spirv(uniform, descriptor_set = 0, binding = 1)] lights: &Lights<
        permutate!(MAX_DIRECTIONAL_LIGHTS),
        permutate!(MAX_CASCADES_PER_LIGHT),
    >,

    #[permutate(texture_format = texture)]
    #[spirv(descriptor_set = 0, binding = 2)]
    point_shadow_textures: &crate::prelude::PointShadowTexture,

    #[permutate(texture_format = array)]
    #[spirv(descriptor_set = 0, binding = 2)]
    point_shadow_textures: &crate::prelude::PointShadowTextureArray,

    #[spirv(descriptor_set = 0, binding = 3)] point_shadow_textures_sampler: &Sampler,

    #[permutate(texture_format = texture)]
    #[spirv(descriptor_set = 0, binding = 4)]
    directional_shadow_textures: &crate::prelude::DirectionalShadowTexture,

    #[permutate(texture_format = array)]
    #[spirv(descriptor_set = 0, binding = 4)]
    directional_shadow_textures: &crate::prelude::DirectionalShadowTextureArray,

    #[spirv(descriptor_set = 0, binding = 5)] directional_shadow_textures_sampler: &Sampler,

    #[permutate(buffer_format = uniform)]
    #[spirv(uniform, descriptor_set = 0, binding = 6)]
    point_lights: &crate::prelude::PointLightsUniform,

    #[permutate(buffer_format = storage)]
    #[spirv(storage_buffer, descriptor_set = 0, binding = 6)]
    point_lights: &crate::prelude::PointLightsStorage,

    #[permutate(buffer_format = uniform)]
    #[spirv(uniform, descriptor_set = 0, binding = 7)]
    cluster_light_index_lists: &crate::prelude::ClusterLightIndexListsUniform,

    #[permutate(buffer_format = storage)]
    #[spirv(storage_buffer, descriptor_set = 0, binding = 7)]
    cluster_light_index_lists: &crate::prelude::ClusterLightIndexListsStorage,

    #[permutate(buffer_format = uniform)]
    #[spirv(uniform, descriptor_set = 0, binding = 8)]
    cluster_offsets_and_counts: &crate::prelude::ClusterOffsetsAndCountsUniform,

    #[permutate(buffer_format = storage)]
    #[spirv(storage_buffer, descriptor_set = 0, binding = 8)]
    cluster_offsets_and_counts: &crate::prelude::ClusterOffsetsAndCountsStorage,

    #[spirv(uniform, descriptor_set = 0, binding = 10)] fog: &Fog,

    #[spirv(descriptor_set = 0, binding = 11)] environment_map_diffuse: &TextureCube,
    #[spirv(descriptor_set = 0, binding = 12)] environment_map_specular: &TextureCube,
    #[spirv(descriptor_set = 0, binding = 13)] environment_map_sampler: &Sampler,

    #[spirv(uniform, descriptor_set = 1, binding = 0)] material: &BaseMaterial,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 1)]
    base_color_texture: &BaseColorTexture,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 2)]
    base_color_sampler: &Sampler,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 3)]
    emissive_texture: &EmissiveTexture,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 4)]
    emissive_sampler: &Sampler,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 5)]
    metallic_roughness_texture: &MetallicRoughnessTexture,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 6)]
    metallic_roughness_sampler: &Sampler,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 7)]
    occlusion_texture: &OcclusionTexture,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 8)]
    occlusion_sampler: &Sampler,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 9)]
    normal_map_texture: &NormalMapTexture,

    #[allow(unused_variables)]
    #[spirv(descriptor_set = 1, binding = 10)]
    normal_map_sampler: &Sampler,

    #[spirv(uniform, descriptor_set = 2, binding = 0)] mesh: &Mesh,

    #[allow(unused_variables)]
    #[spirv(front_facing)]
    in_is_front: bool,

    #[spirv(position)] in_frag_coord: Vec4,
    in_world_position: Vec4,
    in_world_normal: Vec3,
    #[allow(unused_variables)] in_uv: Vec2,
    #[permutate(tangent = some)] in_tangent: Vec4,
    #[permutate(color = some)] in_color: Vec4,
    output_color: &mut Vec4,
) {
    #[permutate(texture_format = texture)]
    type _PointShadow = crate::prelude::PointShadowTexture;
    #[permutate(texture_format = array)]
    type _PointShadow = crate::prelude::PointShadowTextureArray;

    #[permutate(texture_format = texture)]
    type _DirectionalShadow = crate::prelude::DirectionalShadowTexture;
    #[permutate(texture_format = array)]
    type _DirectionalShadow = crate::prelude::DirectionalShadowTextureArray;

    #[permutate(buffer_format = uniform)]
    type _PointLights = crate::prelude::PointLightsUniform;
    #[permutate(buffer_format = storage)]
    type _PointLights = crate::prelude::PointLightsStorage;

    #[permutate(buffer_format = uniform)]
    type _ClusterLightIndexLists = crate::prelude::ClusterLightIndexListsUniform;
    #[permutate(buffer_format = storage)]
    type _ClusterLightIndexLists = crate::prelude::ClusterLightIndexListsStorage;

    #[permutate(buffer_format = uniform)]
    type _ClusterOffsetsAndCounts = crate::prelude::ClusterOffsetsAndCountsUniform;
    #[permutate(buffer_format = storage)]
    type _ClusterOffsetsAndCounts = crate::prelude::ClusterOffsetsAndCountsStorage;

    #[permutate(blend_mode = multiply)]
    type _PremultiplyAlpha = crate::prelude::Multiply;
    #[permutate(blend_mode = blend_premultiplied_alpha)]
    type _PremultiplyAlpha = crate::prelude::BlendPremultipliedAlpha;
    #[permutate(blend_mode = none)]
    type _PremultiplyAlpha = ();

    #[permutate(environment_map = some)]
    type _EnvironmentMap = ();
    #[permutate(environment_map = none)]
    type _EnvironmentMap = ();

    #[permutate(cluster_debug = debug_z_slices)]
    type _ClusterDebug = crate::prelude::DebugZSlices;
    #[permutate(cluster_debug = debug_cluster_light_complexity)]
    type _ClusterDebug = crate::prelude::DebugClusterLightComplexity;
    #[permutate(cluster_debug = debug_cluster_coherency)]
    type _ClusterDebug = crate::prelude::DebugClusterCoherency;
    #[permutate(cluster_debug = none)]
    type _ClusterDebug = ();

    #[permutate(directional_light_shadow_map_debug = some)]
    type _DirectionalLightShadowMapDebug = crate::prelude::DebugCascades;
    #[permutate(directional_light_shadow_map_debug = none)]
    type _DirectionalLightShadowMapDebug = ();

    let vertex_position = in_world_position;
    let vertex_normal = in_world_normal;

    #[permutate(uv = some)]
    let vertex_uv = &in_uv;

    #[permutate(color = some)]
    let vertex_color = &in_color;

    #[permutate(color = some)]
    let vertex_tangent = &in_tangent;

    *output_color = material.base.base_color;

    #[permutate(color = some)]
    *output_color *= *vertex_color;

    #[permutate(uv = some)]
    if (material.base.flags & STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0 {
        *output_color =
            *output_color * base_color_texture.sample::<f32, Vec4>(*base_color_sampler, *vertex_uv);
    }

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if material.base.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT == 0 {
        // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
        // the material members
        let mut pbr_input = PbrInput::default();

        pbr_input.material.base_color = *output_color;
        pbr_input.material.reflectance = material.base.reflectance;
        pbr_input.material.flags = material.base.flags;
        pbr_input.material.alpha_cutoff = material.base.alpha_cutoff;

        // TODO use .a for exposure compensation in HDR
        let emissive = material.base.emissive;

        #[permutate(uv = some)]
        let emissive = if (material.base.flags & STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0
        {
            (emissive.truncate()
                * emissive_texture
                    .sample::<f32, Vec4>(*emissive_sampler, *vertex_uv)
                    .truncate())
            .extend(1.0)
        } else {
            emissive
        };

        pbr_input.material.emissive = emissive;

        #[allow(unused_mut)]
        let mut metallic = material.base.metallic;

        #[allow(unused_mut)]
        let mut perceptual_roughness = material.base.perceptual_roughness;

        #[permutate(uv = some)]
        if (material.base.flags & STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0 {
            let metallic_roughness = metallic_roughness_texture
                .sample::<f32, Vec4>(*metallic_roughness_sampler, *vertex_uv);
            // Sampling from GLTF standard channels for now
            metallic = metallic * metallic_roughness.z;
            perceptual_roughness = perceptual_roughness * metallic_roughness.y;
        }

        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        #[allow(unused_mut)]
        let mut occlusion: f32 = 1.0;

        #[permutate(uv = some)]
        if (material.base.flags & STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0 {
            occlusion = occlusion_texture
                .sample::<f32, Vec4>(*occlusion_sampler, *vertex_uv)
                .x;
        }

        pbr_input.occlusion = occlusion;

        pbr_input.frag_coord = in_frag_coord;
        pbr_input.world_position = vertex_position;
        pbr_input.world_normal = vertex_normal;

        #[permutate(tangent = some)]
        {
            #[permutate(normal_map = some)]
            {
                // NOTE: When NOT using normal-mapping, if looking at the back face of a double-sided
                // material, the normal needs to be inverted. This is a branchless version of that.
                pbr_input.world_normal =
                    (if !(material.base.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0
                        || in_is_front
                    {
                        1.0
                    } else {
                        0.0
                    } * 2.0
                        - 1.0)
                        * pbr_input.world_normal;
            }
        }

        pbr_input.is_orthographic = view.projection.w_axis.w == 1.0;

        #[permutate(uv = some)]
        {
            #[permutate(tangent = some)]
            {
                #[permutate(normal_map = some)]
                // NOTE: The mikktspace method of normal mapping explicitly requires that these NOT be
                // normalized nor any Gram-Schmidt applied to ensure the vertex normal is orthogonal to the
                // vertex tangent! Do not change this code unless you really know what you are doing.
                // http://www.mikktspace.com/
                let t: Vec3 = vertex_tangent.truncate();
                let b: Vec3 = vertex_tangent.w * pbr_input.n.cross(t);

                // Nt is the tangent-space normal.
                let mut nt = normal_map_texture
                    .sample::<f32, Vec4>(*normal_map_sampler, *vertex_uv)
                    .truncate();
                if (material.base.flags & STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP) != 0 {
                    // Only use the xy components and derive z for 2-component normal maps.
                    nt = (nt.truncate() * 2.0 - 1.0).extend(0.0);
                    nt.z = (1.0 - nt.x * nt.x - nt.y * nt.y).sqrt();
                } else {
                    nt = nt * 2.0 - 1.0;
                }
                // Normal maps authored for DirectX require flipping the y component
                if (material.base.flags & STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y) != 0 {
                    nt.y = -nt.y;
                }
                // NOTE: The mikktspace method of normal mapping applies maps the tangent-space normal from
                // the normal map texture in this way to be an EXACT inverse of how the normal map baker
                // calculates the normal maps so there is no error introduced. Do not change this code
                // unless you really know what you are doing.
                // http://www.mikktspace.com/
                pbr_input.n = nt.x * t + nt.y * b + nt.z * pbr_input.n;
            }

            pbr_input.n = pbr_input.world_normal.normalize();
        }

        pbr_input.v = view.calculate_view(vertex_position, pbr_input.is_orthographic);
        pbr_input.occlusion = occlusion;

        pbr_input.flags = mesh.flags;

        *output_color = pbr_input
            .pbr::<
                permutate!(MAX_DIRECTIONAL_LIGHTS),
                permutate!(MAX_CASCADES_PER_LIGHT),
                _PointLights,
                _DirectionalShadow,
                _PointShadow,
                _ClusterLightIndexLists,
                _ClusterOffsetsAndCounts,
                _EnvironmentMap,
                _ClusterDebug,
                _DirectionalLightShadowMapDebug
            >(
                view,
                mesh,
                lights,
                point_lights,
                cluster_light_index_lists,
                cluster_offsets_and_counts,
                directional_shadow_textures,
                directional_shadow_textures_sampler,
                point_shadow_textures,
                point_shadow_textures_sampler,
                environment_map_diffuse,
                environment_map_specular,
                environment_map_sampler,
            );
    } else {
        *output_color = material.base.alpha_discard(*output_color);
    }

    // fog
    if fog.mode != FOG_MODE_OFF
        && (material.base.flags & STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT) != 0
    {
        *output_color = apply_fog(
            fog,
            lights,
            *output_color,
            in_world_position.truncate(),
            view.world_position,
        )
    }

    #[permutate(tonemap = some)]
    *output_color =
        crate::prelude::reinhard_luminance(output_color.truncate()).extend(output_color.w);

    #[permutate(deband = some)]
    *output_color = {
        let mut output_rgb = output_color.truncate();
        output_rgb = powsafe(output_rgb, 1.0 / 2.2);
        output_rgb =
            output_rgb + crate::prelude::screen_space_dither(in_frag_coord.truncate().truncate());
        // This conversion back to linear space is required because our output texture format is
        // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
        output_rgb = powsafe(output_rgb, 2.2);
        output_rgb.extend(output_color.w)
    };

    #[permutate(premultiply_alpha = some)]
    *output_color = _PremultiplyAlpha::premultiply_alpha(material.base.flags, *output_color);
}
