# bevy-pbr-rust

A Rust reimplementation of `bevy_pbr`'s WGSL shaders.

Shader def conditionals are implemented using compile-time trait generics, and entrypoint permutations are generated via macro annotations.

At time of writing, `rust-gpu` only supports read-write access to storage buffers,
which renders it implementation incompatible with the read-only buffers bevy uses to store light and cluster data on supported platforms.

As such, consuming bevy applications should make sure to force storage buffers off via `WgpuSettings`.

