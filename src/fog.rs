// Fog formulas adapted from:
// https://learn.microsoft.com/en-us/windows/win32/direct3d9/fog-formulas
// https://catlikecoding.com/unity/tutorials/rendering/part-14/
// https://iquilezles.org/articles/fog/ (Atmospheric Fog and Scattering)

use rust_gpu_bridge::glam::{Vec3, Vec4};
use spirv_std::num_traits::Float;

// Important: These must be kept in sync with `fog.rs`
pub const FOG_MODE_OFF: u32 = 0;
pub const FOG_MODE_LINEAR: u32 = 1;
pub const FOG_MODE_EXPONENTIAL: u32 = 2;
pub const FOG_MODE_EXPONENTIAL_SQUARED: u32 = 3;
pub const FOG_MODE_ATMOSPHERIC: u32 = 4;

#[repr(C, packed(4))]
pub struct Fog {
    pub base_color: Vec4,
    pub directional_light_color: Vec4,
    // `be` and `bi` are allocated differently depending on the fog mode
    //
    // For Linear Fog:
    //     be.x = start, be.y = end
    // For Exponential and ExponentialSquared Fog:
    //     be.x = density
    // For Atmospheric Fog:
    //     be = per-channel extinction density
    //     bi = per-channel inscattering density
    //pub be: Vec3,
    pub be_x: f32,
    pub be_y: f32,
    pub be_z: f32,
    pub directional_light_exponent: f32,
    //pub bi: Vec3,
    pub bi_x: f32,
    pub bi_y: f32,
    pub bi_z: f32,
    pub mode: u32,
}

impl Fog {
    pub fn scattering_adjusted_fog_color(&self, scattering: Vec3) -> Vec4 {
        if self.directional_light_color.w > 0.0 {
            return (self.base_color.truncate()
                + scattering
                    * self.directional_light_color.truncate()
                    * self.directional_light_color.w)
                .extend(self.base_color.w);
        } else {
            return self.base_color;
        }
    }

    pub fn linear_fog(&self, input_color: Vec4, distance: f32, scattering: Vec3) -> Vec4 {
        let mut fog_color = self.scattering_adjusted_fog_color(scattering);
        let start = self.be_x;
        let end = self.be_y;
        fog_color.w *= 1.0 - ((end - distance) / (end - start)).clamp(0.0, 1.0);
        return input_color
            .truncate()
            .lerp(fog_color.truncate(), fog_color.w)
            .extend(input_color.w);
    }

    pub fn exponential_fog(&self, input_color: Vec4, distance: f32, scattering: Vec3) -> Vec4 {
        let mut fog_color = self.scattering_adjusted_fog_color(scattering);
        let density = self.be_x;
        fog_color.w *= 1.0 - 1.0 / (distance * density).exp();
        return input_color
            .truncate()
            .lerp(fog_color.truncate(), fog_color.w)
            .extend(input_color.w);
    }

    pub fn exponential_squared_fog(&self, input_color: Vec4, distance: f32, scattering: Vec3) -> Vec4 {
        let mut fog_color = self.scattering_adjusted_fog_color(scattering);
        let distance_times_density = distance * self.be_x;
        fog_color.w *= 1.0 - 1.0 / (distance_times_density * distance_times_density).exp();
        return input_color
            .truncate()
            .lerp(fog_color.truncate(), fog_color.w)
            .extend(input_color.w);
    }

    pub fn atmospheric_fog(&self, input_color: Vec4, distance: f32, scattering: Vec3) -> Vec4 {
        let fog_color = self.scattering_adjusted_fog_color(scattering);
        let extinction_factor = 1.0 - 1.0 / (distance * Vec3::new(self.be_x, self.be_y, self.be_z)).exp();
        let inscattering_factor = 1.0 - 1.0 / (distance * Vec3::new(self.bi_x, self.bi_y, self.bi_z)).exp();
        return (input_color.truncate() * (1.0 - extinction_factor * fog_color.w)
            + fog_color.truncate() * inscattering_factor * fog_color.w)
            .extend(input_color.w);
    }
}
