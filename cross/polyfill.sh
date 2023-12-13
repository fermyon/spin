#!/bin/sh

ROOT="$(dirname "$0")"
echo "ROOT=$ROOT"
ARM_NEON_PATH="$(aarch64-linux-musl-gcc -E "$ROOT/test_vld1q_s8_x4.c" | grep -m1 "arm_neon.h" | sed -En 's|.*"(/usr/local/[^"]*/arm_neon.h)".*|\1|p')"
echo "ARM_NEON_PATH=$ARM_NEON_PATH"
if command -v aarch64-linux-musl-gcc > /dev/null; then
    if ! aarch64-linux-musl-gcc -Werror=implicit-function-declaration -c -o /dev/null "$ROOT/test_vld1q_u8_x4.c"; then
        echo "Polyfilling vld1q_u8_x4"
        cat "$ROOT/polyfill_vld1q_u8_x4.h" >> $ARM_NEON_PATH
    fi
    if ! aarch64-linux-musl-gcc -Werror=implicit-function-declaration -c -o /dev/null "$ROOT/test_vld1q_s8_x4.c"; then
        echo "Polyfilling vld1q_s8_x4"
        cat "$ROOT/polyfill_vld1q_s8_x4.h" >> $ARM_NEON_PATH
    fi
fi