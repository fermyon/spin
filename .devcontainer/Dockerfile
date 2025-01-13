# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.224.2/containers/rust/.devcontainer/base.Dockerfile

# [Choice] Debian OS version (use bullseye on local arm64/Apple Silicon): buster, bullseye
ARG VARIANT="buster"
FROM mcr.microsoft.com/vscode/devcontainers/rust:0-${VARIANT}

# [Optional] Uncomment this section to install additional packages.
# RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
#     && apt-get -y install --no-install-recommends <your-package-list-here>

# Go installation, see https://go.dev/doc/install
ARG GO_URL="https://go.dev/dl/go1.22.0.linux-amd64.tar.gz"
RUN curl -sL "$GO_URL" | tar -xzf - -C /usr/local
ENV PATH "$PATH:/usr/local/go/bin"

# TinyGo installation, see https://tinygo.org/getting-started/install/linux/ for instructions
ARG TINYGO_URL="https://github.com/tinygo-org/tinygo/releases/download/v0.35.0/tinygo_0.35.0_amd64.deb"
RUN curl -sL "$TINYGO_URL" -o tinygo_amd64.deb && dpkg -i tinygo_amd64.deb && rm tinygo_amd64.deb

# Install the gopls Go Language Server, see https://github.com/golang/tools/tree/master/gopls
RUN go install golang.org/x/tools/gopls@latest

# Install additional rust components (rust-analyzer)
ARG RUST_ANALYZER_URL="https://github.com/rust-lang/rust-analyzer/releases/download/2022-04-11/rust-analyzer-x86_64-unknown-linux-gnu.gz"
RUN curl -sL "$RUST_ANALYZER_URL" | gunzip -c - > /usr/local/bin/rust-analyzer && chmod +x /usr/local/bin/rust-analyzer

# Install Grain, see https://grain-lang.org/docs/getting_grain
ARG GRAIN_URL="https://github.com/grain-lang/grain/releases/download/grain-v0.4.7/grain-linux-x64"
RUN curl -sL "$GRAIN_URL" -o /usr/local/bin/grain && chmod +x /usr/local/bin/grain

# Install AssemblyScript (Node.js dependency), see https://www.assemblyscript.org/getting-started.html
ARG NODEJS_URL="https://nodejs.org/dist/v17.9.0/node-v17.9.0-linux-x64.tar.xz"
RUN curl -sL "$NODEJS_URL" | tar -xJf - -C /usr/local --strip-components=1 --no-same-owner
RUN npm install -g assemblyscript
