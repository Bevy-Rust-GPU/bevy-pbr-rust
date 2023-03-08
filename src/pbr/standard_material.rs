use spirv_std::{arch::kill, glam::Vec4};

pub const STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT: u32 = 1;
pub const STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT: u32 = 2;
pub const STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT: u32 = 4;
pub const STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT: u32 = 8;
pub const STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT: u32 = 16;
pub const STANDARD_MATERIAL_FLAGS_UNLIT_BIT: u32 = 32;
pub const STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP: u32 = 64;
pub const STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y: u32 = 128;
pub const STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT: u32 = 256;
pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS: u32 = 3758096384; // (0b111u32 << 29)
pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32 = 0; // (0u32 << 29)
pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32 = 536870912; // (1u32 << 29)
pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32 = 1073741824; // (2u32 << 29)
pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED: u32 = 1610612736; // (3u32 << 29)
pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD: u32 = 2147483648; // (4u32 << 29)
pub const STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MULTIPLY: u32 = 2684354560; // (5u32 << 29)
                                                                         // ↑ To calculate/verify the values above, use the following playground:
                                                                         // https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=7792f8dd6fc6a8d4d0b6b1776898a7f4

#[repr(C)]
pub struct StandardMaterial {
    pub base_color: Vec4,
    pub emissive: Vec4,
    pub perceptual_roughness: f32,
    pub metallic: f32,
    pub reflectance: f32,
    // 'flags' is a bit field indicating various options. u32 is 32 bits so we have up to 32 options.
    pub flags: u32,
    pub alpha_cutoff: f32,
}

impl Default for StandardMaterial {
    fn default() -> Self {
        StandardMaterial {
            base_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            emissive: Vec4::new(0.0, 0.0, 0.0, 1.0),
            perceptual_roughness: 0.089,
            metallic: 0.01,
            reflectance: 0.5,
            flags: STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE,
            alpha_cutoff: 0.5,
        }
    }
}

impl StandardMaterial {
    pub fn alpha_discard(&self, output_color: Vec4) -> Vec4 {
        let mut color = output_color;
        let alpha_mode = self.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
        if alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
            // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
            color.w = 1.0;
        } else if alpha_mode == STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK {
            if color.w >= self.alpha_cutoff {
                // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
                color.w = 1.0;
            } else {
                // NOTE: output_color.a < in.material.alpha_cutoff should not is not rendered
                // NOTE: This and any other discards mean that early-z testing cannot be done!
                kill();
            }
        }
        return color;
    }
}
