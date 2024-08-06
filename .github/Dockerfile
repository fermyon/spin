FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
  ca-certificates \
  git

ARG TARGETARCH
ARG TARGETOS
COPY spin-${TARGETOS}-${TARGETARCH} /usr/local/bin/spin

ENTRYPOINT [ "/usr/local/bin/spin" ]
