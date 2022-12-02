ARG VARIANT="bullseye"
FROM rust:1-${VARIANT}

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    # Remove imagemagick due to https://security-tracker.debian.org/tracker/CVE-2019-10131
    && apt-get purge -y imagemagick imagemagick-6-common

# [Optional] Uncomment this section to install additional OS packages.
# RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
#     && apt-get -y install --no-install-recommends <your-package-list-here>

# Install additional rust components (rust-analyzer)
ARG RUST_ANALYZER_URL="https://github.com/rust-lang/rust-analyzer/releases/download/2022-04-11/rust-analyzer-x86_64-unknown-linux-gnu.gz"
RUN curl -sL "$RUST_ANALYZER_URL" | gunzip -c - > /usr/local/bin/rust-analyzer && chmod +x /usr/local/bin/rust-analyzer

RUN rustup target add wasm32-wasi
