FROM ubuntu:22.04

WORKDIR /root
RUN apt-get update && apt-get install -y wget sudo xz-utils gcc git pkg-config

# nodejs
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | sudo -E bash -
RUN apt-get install -y nodejs npm

# golang
RUN wget https://go.dev/dl/go1.19.5.linux-amd64.tar.gz && \
    rm -rf /usr/local/go && tar -C /usr/local -xzf go1.19.5.linux-amd64.tar.gz
ENV PATH="$PATH:/usr/local/go/bin"

# tinygo
RUN wget https://github.com/tinygo-org/tinygo/releases/download/v0.25.0/tinygo_0.25.0_amd64.deb && \
    sudo dpkg -i tinygo_0.25.0_amd64.deb && \
    tinygo env

# zig
RUN wget https://ziglang.org/download/0.10.0/zig-linux-x86_64-0.10.0.tar.xz && \
    tar -xf zig-linux-x86_64-0.10.0.tar.xz
ENV PATH="$PATH:/root/zig-linux-x86_64-0.10.0"

# grain
RUN wget https://github.com/grain-lang/grain/releases/download/grain-v0.5.4/grain-linux-x64 && \
    mv grain-linux-x64 /usr/local/bin/grain && chmod +x /usr/local/bin/grain

# spin
RUN wget https://github.com/fermyon/spin/releases/download/canary/spin-canary-linux-amd64.tar.gz && \
    tar -xvf spin-canary-linux-amd64.tar.gz && \
    ls -ltr && \
    mv spin /usr/local/bin/spin


# # rust
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN url="https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"; \
    wget "$url"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --default-toolchain stable --default-host x86_64-unknown-linux-gnu; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version; \
    rustup target add wasm32-wasi;

## check versions
RUN tinygo version
RUN go version
RUN grain --version
RUN spin --version
RUN zig version
RUN rustc --version
RUN node --version

WORKDIR /e2e-tests
COPY . .

CMD cargo test spinup_tests --no-fail-fast -- --nocapture
