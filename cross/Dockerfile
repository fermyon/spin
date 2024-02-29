FROM ghcr.io/cross-rs/aarch64-unknown-linux-musl@sha256:7ad32efcae37c178fed63da301ff982d2430f15fc7d39e6c0b28124e14fac394

## as per analysis done in https://github.com/fermyon/spin/pull/2287#issuecomment-1970145410 we need
## to disable the cmake config in cross-rs. Setting CROSS_SYSROOT=/ seems to have done the trick

ENV CROSS_SYSROOT=/

RUN --mount=type=bind,source=/,target=/polyfill /polyfill/polyfill.sh


