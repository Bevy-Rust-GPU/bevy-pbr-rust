use core::ops::{Index, Mul};

use spirv_std::glam::UVec4;

pub trait ClusterLightIndexLists {
    fn get_light_id(&self, index: u32) -> u32;
}

pub type ClusterLightIndexListsUniform<'a> = &'a [UVec4; 1024];

impl ClusterLightIndexLists for ClusterLightIndexListsUniform<'_> {
    fn get_light_id(&self, index: u32) -> u32 {
        // The index is correct but in cluster_light_index_lists we pack 4 u8s into a u32
        // This means the index into cluster_light_index_lists is index / 4
        let v = self[(index >> 4) as usize];
        let indices = match ((index >> 2) & ((1 << 2) - 1)) as usize {
            0 => v.x,
            1 => v.y,
            2 => v.z,
            3 => v.w,
            _ => panic!(),
        };
        // And index % 4 gives the sub-index of the u8 within the u32 so we shift by 8 * sub-index
        (indices >> (8.mul(index & ((1 << 2) - 1)))) & ((1 << 8) - 1)
    }
}

pub type ClusterLightIndexListsStorage<'a> = &'a [u32];

impl ClusterLightIndexLists for ClusterLightIndexListsStorage<'_> {
    fn get_light_id(&self, index: u32) -> u32 {
        *unsafe { self.index(index as usize) }
    }
}
