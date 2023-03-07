use spirv_std::glam::Mat4;

#[repr(C)]
pub struct SkinnedMesh {
    pub data: [Mat4; 256],
}
