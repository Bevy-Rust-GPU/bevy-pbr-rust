use core::ops::Index;

use spirv_std::glam::{UVec3, UVec4};

use crate::prelude::CLUSTER_COUNT_SIZE;

pub trait ClusterOffsetsAndCounts {
    fn unpack(&self, cluster_index: u32) -> UVec3;
}

pub type ClusterOffsetsAndCountsUniform<'a> = &'a [UVec4; 1024];

impl ClusterOffsetsAndCounts for ClusterOffsetsAndCountsUniform<'_> {
    fn unpack(&self, cluster_index: u32) -> UVec3 {
        let v = self[(cluster_index >> 2) as usize];
        let i = cluster_index & ((1 << 2) - 1);
        let offset_and_counts = match i {
            0 => v.x,
            1 => v.y,
            2 => v.z,
            3 => v.w,
            _ => panic!(),
        };
        //  [ 31     ..     18 | 17      ..      9 | 8       ..     0 ]
        //  [      offset      | point light count | spot light count ]
        UVec3::new(
            (offset_and_counts >> (CLUSTER_COUNT_SIZE * 2))
                & ((1 << (32 - (CLUSTER_COUNT_SIZE * 2))) - 1),
            (offset_and_counts >> CLUSTER_COUNT_SIZE) & ((1 << CLUSTER_COUNT_SIZE) - 1),
            offset_and_counts & ((1 << CLUSTER_COUNT_SIZE) - 1),
        )
    }
}

pub type ClusterOffsetsAndCountsStorage<'a> = &'a [UVec4];

impl ClusterOffsetsAndCounts for ClusterOffsetsAndCountsStorage<'_> {
    fn unpack(&self, cluster_index: u32) -> UVec3 {
        unsafe { self.index(cluster_index as usize) }.truncate()
    }
}
