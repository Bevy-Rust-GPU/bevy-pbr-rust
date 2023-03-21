use rust_gpu_bridge::{glam::Mat3, Exp2, Log2, Mix, Pow, Saturate};
use spirv_std::{
    glam::{Vec2, Vec3},
    Sampler,
};

use crate::prelude::Texture3d;

pub trait SampleCurrentLut {
    fn sample_current_lut(texture: &Texture3d, sampler: &Sampler, p: Vec3) -> Vec3;
}

pub enum TonemapMethodAgx {}

impl SampleCurrentLut for TonemapMethodAgx {
    fn sample_current_lut(dt_lut_texture: &Texture3d, dt_lut_sampler: &Sampler, p: Vec3) -> Vec3 {
        dt_lut_texture
            .sample_by_lod::<f32>(*dt_lut_sampler, p, 0.0)
            .truncate()
    }
}

pub enum TonemapMethodTonyMcMapFace {}

impl SampleCurrentLut for TonemapMethodTonyMcMapFace {
    fn sample_current_lut(dt_lut_texture: &Texture3d, dt_lut_sampler: &Sampler, p: Vec3) -> Vec3 {
        dt_lut_texture
            .sample_by_lod::<f32>(*dt_lut_sampler, p, 0.0)
            .truncate()
    }
}

pub enum TonemapMethodBlenderFilmic {}

impl SampleCurrentLut for TonemapMethodBlenderFilmic {
    fn sample_current_lut(dt_lut_texture: &Texture3d, dt_lut_sampler: &Sampler, p: Vec3) -> Vec3 {
        dt_lut_texture
            .sample_by_lod::<f32>(*dt_lut_sampler, p, 0.0)
            .truncate()
    }
}

impl SampleCurrentLut for () {
    fn sample_current_lut(_: &Texture3d, _: &Sampler, _: Vec3) -> Vec3 {
        Vec3::new(1.0, 0.0, 1.0)
    }
}

// --------------------------------------
// --- SomewhatBoringDisplayTransform ---
// --------------------------------------
// By Tomasz Stachowiak

pub fn rgb_to_ycbcr(col: Vec3) -> Vec3 {
    let m = Mat3::from_cols_array(&[
        0.2126, 0.7152, 0.0722, -0.1146, -0.3854, 0.5, 0.5, -0.4542, -0.0458,
    ]);
    return m * col;
}

pub fn ycbcr_to_rgb(col: Vec3) -> Vec3 {
    let m = Mat3::from_cols_array(&[1.0, 0.0, 1.5748, 1.0, -0.1873, -0.4681, 1.0, 1.8556, 0.0]);
    return Vec3::ZERO.max(m * col);
}

pub fn tonemap_curve(v: f32) -> f32 {
    #[cfg(not(feature = "never"))]
    {
        // Large linear part in the lows, but compresses highs.
        let c = v + v * v + 0.5 * v * v * v;
        return c / (1.0 + c);
    }

    #[cfg(feature = "never")]
    {
        return 1.0 - (-v).exp();
    }
}

pub fn tonemap_curve3(v: Vec3) -> Vec3 {
    return Vec3::new(tonemap_curve(v.x), tonemap_curve(v.y), tonemap_curve(v.z));
}

pub fn somewhat_boring_display_transform(col: Vec3) -> Vec3 {
    let mut col = col;
    let ycbcr = rgb_to_ycbcr(col);

    let bt = tonemap_curve(Vec2::new(ycbcr.y, ycbcr.z).length() * 2.4);
    let mut desat = ((bt - 0.7) * 0.8).max(0.0);
    desat *= desat;

    let desat_col = col.mix(Vec3::splat(ycbcr.x), Vec3::splat(desat));

    let tm_luma = tonemap_curve(ycbcr.x);
    let tm0 = col * 0.0_f32.max(tm_luma / 1e-5_f32.max(tonemapping_luminance(col)));
    let final_mult = 0.97;
    let tm1 = tonemap_curve3(desat_col);

    col = tm0.mix(tm1, Vec3::splat(bt * bt));

    return col * final_mult;
}

// ------------------------------------------
// ------------- Tony McMapface -------------
// ------------------------------------------
// By Tomasz Stachowiak
// https://github.com/h3r2tic/tony-mc-mapface

const TONY_MC_MAPFACE_LUT_EV_RANGE: Vec2 = Vec2::new(-13.0, 8.0);
const TONY_MC_MAPFACE_LUT_DIMS: f32 = 48.0;

pub fn tony_mc_mapface_lut_range_encode(x: Vec3) -> Vec3 {
    return x / (x + 1.0);
}

pub fn sample_tony_mc_mapface_lut<S: SampleCurrentLut>(
    texture: &Texture3d,
    sampler: &Sampler,
    stimulus: Vec3,
) -> Vec3 {
    let range = tony_mc_mapface_lut_range_encode(
        Vec3::new(
            TONY_MC_MAPFACE_LUT_EV_RANGE.x,
            TONY_MC_MAPFACE_LUT_EV_RANGE.y,
            TONY_MC_MAPFACE_LUT_EV_RANGE.y,
        )
        .exp2(),
    )
    .truncate();
    let normalized = (tony_mc_mapface_lut_range_encode(stimulus) - range.x) / (range.y - range.x);
    let uv = (normalized
        * ((TONY_MC_MAPFACE_LUT_DIMS - 1.0) as f32 / (TONY_MC_MAPFACE_LUT_DIMS) as f32)
        + 0.5 / (TONY_MC_MAPFACE_LUT_DIMS) as f32)
        .saturate();

    S::sample_current_lut(texture, sampler, uv)
}

// ---------------------------------
// ---------- ACES Fitted ----------
// ---------------------------------

// Same base implementation that Godot 4.0 uses for Tonemap ACES.

// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl

// The code in this file was originally written by Stephen Hill (@self_shadow), who deserves all
// credit for coming up with this fit and implementing it. Buy him a beer next time you see him. :)

pub fn rrt_and_odt_fit(v: Vec3) -> Vec3 {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

pub fn aces_fitted(color: Vec3) -> Vec3 {
    let mut color = color;

    // sRGB => XYZ => D65_2_D60 => AP1 => RRT_SAT
    let rgb_to_rrt = Mat3::from_cols(
        Vec3::new(0.59719, 0.35458, 0.04823),
        Vec3::new(0.07600, 0.90834, 0.01566),
        Vec3::new(0.02840, 0.13383, 0.83777),
    );

    // ODT_SAT => XYZ => D60_2_D65 => sRGB
    let odt_to_rgb = Mat3::from_cols(
        Vec3::new(1.60475, -0.53108, -0.07367),
        Vec3::new(-0.10208, 1.10813, -0.00605),
        Vec3::new(-0.00327, -0.07276, 1.07602),
    );

    color = rgb_to_rrt * color;

    // Apply RRT and ODT
    color = rrt_and_odt_fit(color);

    color = odt_to_rgb * color;

    // Clamp to [0, 1]
    color = color.saturate();

    return color;
}

// -------------------------------
// ------------- AgX -------------
// -------------------------------
// By Troy Sobotka
// https://github.com/MrLixm/AgXc
// https://github.com/sobotka/AgX

// pow() but safe for NaNs/negatives
pub fn powsafe(color: Vec3, power: f32) -> Vec3 {
    return (color).abs().pow(Vec3::splat(power)) * color.signum();
}

/*
    Increase color saturation of the given color data.
    :param color: expected sRGB primaries input
    :param saturationAmount: expected 0-1 range with 1=neutral, 0=no saturation.
    -- ref[2] [4]
*/
pub fn saturation(color: Vec3, saturation_amount: f32) -> Vec3 {
    let luma = tonemapping_luminance(color);
    return Vec3::splat(luma).mix(color, Vec3::splat(saturation_amount));
}

/*
    Output log domain encoded data.
    Similar to OCIO lg2 AllocationTransform.
    ref[0]
*/
pub fn convert_open_domain_to_normalized_log2(
    color: Vec3,
    minimum_ev: f32,
    maximum_ev: f32,
) -> Vec3 {
    let in_midgrey = 0.18;

    // remove negative before log transform
    let mut color = Vec3::ZERO.max(color);

    // avoid infinite issue with log -- ref[1]
    color.x = if color.x < 0.00003051757 {
        0.00001525878 + color.x
    } else {
        color.x
    };
    color.y = if color.y < 0.00003051757 {
        0.00001525878 + color.y
    } else {
        color.y
    };
    color.z = if color.z < 0.00003051757 {
        0.00001525878 + color.z
    } else {
        color.z
    };

    (color / in_midgrey)
        .log2()
        .clamp(Vec3::splat(minimum_ev), Vec3::splat(maximum_ev));
    let total_exposure = maximum_ev - minimum_ev;

    return (color - minimum_ev) / total_exposure;
}

// Inverse of above
pub fn convert_normalized_log2_to_open_domain(
    color: Vec3,
    minimum_ev: f32,
    maximum_ev: f32,
) -> Vec3 {
    let mut color = color;
    let in_midgrey = 0.18;
    let total_exposure = maximum_ev - minimum_ev;

    color = (color * total_exposure) + minimum_ev;
    color = Vec3::splat(2.0).pow(color);
    color = color * in_midgrey;

    return color;
}

/*=================
    Main processes
=================*/

// Prepare the data for display encoding. Converted to log domain.
pub fn apply_agx_log(image: Vec3) -> Vec3 {
    let mut image = Vec3::ZERO.max(image); // clamp negatives
    let r = image.dot(Vec3::new(0.84247906, 0.0784336, 0.07922375));
    let g = image.dot(Vec3::new(0.04232824, 0.87846864, 0.07916613));
    let b = image.dot(Vec3::new(0.04237565, 0.0784336, 0.87914297));
    image = Vec3::new(r, g, b);

    image = convert_open_domain_to_normalized_log2(image, -10.0, 6.5);

    image = image.clamp(Vec3::ZERO, Vec3::ONE);
    return image;
}

pub fn apply_lut_3d<S: SampleCurrentLut>(
    texture: &Texture3d,
    sampler: &Sampler,
    image: Vec3,
    block_size: f32,
) -> Vec3 {
    S::sample_current_lut(
        texture,
        sampler,
        image * ((block_size - 1.0) / block_size) + 0.5 / block_size,
    )
}

// -------------------------
// -------------------------
// -------------------------

pub fn sample_blender_filmic_lut<S: SampleCurrentLut>(
    texture: &Texture3d,
    sampler: &Sampler,
    stimulus: Vec3,
) -> Vec3 {
    let block_size = 64.0;
    let normalized = convert_open_domain_to_normalized_log2(stimulus, -11.0, 12.0).saturate();
    apply_lut_3d::<S>(texture, sampler, normalized, block_size)
}

// from https://64.github.io/tonemapping/
// reinhard on RGB oversaturates colors
pub fn tonemapping_reinhard(color: Vec3) -> Vec3 {
    color / (1.0 + color)
}

pub fn tonemapping_reinhard_extended(color: Vec3, max_white: f32) -> Vec3 {
    let numerator = color * (1.0 + (color / Vec3::splat(max_white * max_white)));
    numerator / (1.0 + color)
}

// luminance coefficients from Rec. 709.
// https://en.wikipedia.org/wiki/Rec._709
pub fn tonemapping_luminance(v: Vec3) -> f32 {
    v.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

pub fn tonemapping_change_luminance(c_in: Vec3, l_out: f32) -> Vec3 {
    let l_in = tonemapping_luminance(c_in);
    c_in * (l_out / l_in)
}

pub fn reinhard_luminance(color: Vec3) -> Vec3 {
    let l_old = tonemapping_luminance(color);
    let l_new = l_old / (1.0 + l_old);
    tonemapping_change_luminance(color, l_new)
}

// Source: Advanced VR Rendering, GDC 2015, Alex Vlachos, Valve, Slide 49
// https://media.steampowered.com/apps/valve/2015/Alex_Vlachos_Advanced_VR_Rendering_GDC2015.pdf
pub fn screen_space_dither(frag_coord: Vec2) -> Vec3 {
    let mut dither = Vec3::splat(Vec2::new(171.0, 231.0).dot(frag_coord));
    dither = (dither / Vec3::new(103.0, 71.0, 97.0)).fract();
    (dither - 0.5) / 255.0
}
