# Polyfill for missing SIMD intrinsics in `cross-rs` image for target `aarch64-unknown-linux-musl`

A transitive dependency of spin (`llama.cpp` via `ggml`) uses the `vld1q_s8_x4` and `vld1q_u8_x4` compiler built-in SIMD intrinsics.
These intrinsics are missing for `aarch64` in `gcc < 10.3`, while `cross-rs` ships with `gcc 9` for `aarch64-unknown-linux-musl` as of writing.

The code in this folder does a feature check and patches the `arm_neon.h` header with polyfills if the functions are missing.

See https://github.com/fermyon/spin/issues/1786 for the upstream issue.
