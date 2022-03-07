# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.224.2/containers/rust/.devcontainer/base.Dockerfile

# [Choice] Debian OS version (use bullseye on local arm64/Apple Silicon): buster, bullseye
ARG VARIANT="buster"
FROM mcr.microsoft.com/vscode/devcontainers/rust:0-${VARIANT}

# [Optional] Uncomment this section to install additional packages.
# RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
#     && apt-get -y install --no-install-recommends <your-package-list-here>

ARG BINDLE_URL="https://github.com/deislabs/bindle/releases/download/v0.8.0/bindle-v0.8.0-linux-amd64.tar.gz"
RUN curl -sL "$BINDLE_URL" | tar -xzf - -C /usr/local/bin bindle bindle-server