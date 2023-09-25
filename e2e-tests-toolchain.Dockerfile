FROM ubuntu:22.04

ARG BUILD_SPIN=false
ARG FETCH_SPIN=true
ARG SPIN_VERSION=canary

WORKDIR /root
RUN apt-get update && apt-get install -y wget sudo xz-utils gcc git pkg-config redis clang libicu-dev docker.io

# nodejs
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | sudo -E bash -
RUN apt-get install -y nodejs npm

# golang
RUN wget https://go.dev/dl/go1.20.1.linux-amd64.tar.gz && \
    rm -rf /usr/local/go && tar -C /usr/local -xzf go1.20.1.linux-amd64.tar.gz
ENV PATH="$PATH:/usr/local/go/bin"

# tinygo
RUN wget https://github.com/tinygo-org/tinygo/releases/download/v0.27.0/tinygo_0.27.0_amd64.deb && \
    sudo dpkg -i tinygo_0.27.0_amd64.deb && \
    tinygo env

# zig
RUN wget https://ziglang.org/download/0.10.0/zig-linux-x86_64-0.10.0.tar.xz && \
    tar -xf zig-linux-x86_64-0.10.0.tar.xz
ENV PATH="$PATH:/root/zig-linux-x86_64-0.10.0"

# grain
RUN wget https://github.com/grain-lang/grain/releases/download/grain-v0.5.4/grain-linux-x64 && \
    mv grain-linux-x64 /usr/local/bin/grain && chmod +x /usr/local/bin/grain

# # rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN url="https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init";                        \
    wget "$url";                                                                                                \
    chmod +x rustup-init;                                                                                       \
    ./rustup-init -y --no-modify-path --default-toolchain 1.71 --default-host x86_64-unknown-linux-gnu;         \
    rm rustup-init;                                                                                             \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME;                                                                      \
    rustup --version;                                                                                           \
    cargo --version;                                                                                            \
    rustc --version;                                                                                            \
    rustup target add wasm32-wasi;                                                                              \
    rustup target add wasm32-unknown-unknown;

# swift
RUN wget https://github.com/swiftwasm/swift/releases/download/swift-wasm-5.8-SNAPSHOT-2023-02-24-a/swift-wasm-5.8-SNAPSHOT-2023-02-24-a-ubuntu22.04_x86_64.tar.gz && \
    tar -xf swift-wasm-5.8-SNAPSHOT-2023-02-24-a-ubuntu22.04_x86_64.tar.gz
ENV PATH="$PATH:/root/swift-wasm-5.8-SNAPSHOT-2023-02-24-a/usr/bin"

## check versions
RUN tinygo version;   \
    go version;       \
    zig version;      \
    rustc --version;  \
    node --version;   \
    swift --version;
