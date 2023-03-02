<div align="center">

# `bevy-pbr-rust`

[![Documentation](https://img.shields.io/badge/docs-API-blue)](https://bevy-rust-gpu.github.io/bevy-pbr-rust/)

A Rust reimplementation of `bevy_pbr`'s WGSL shaders.

</div>

## Implementation

Shader def conditionals are implemented using compile-time trait generics, and entrypoint permutations are generated via macro annotations.

## Compatibility

At time of writing, `rust-gpu` only supports read-write access to storage buffers,
which renders it implementation incompatible with the read-only buffers bevy uses to store light and cluster data on supported platforms.

As such, consuming bevy applications should make sure to force storage buffers off via `WgpuSettings`.
This is taken care of automatically if using `bevy-rust-gpu`.

