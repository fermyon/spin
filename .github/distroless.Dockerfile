FROM gcr.io/distroless/static-debian12

ARG TARGETARCH
ARG TARGETOS
COPY spin-static-${TARGETOS}-${TARGETARCH} /usr/local/bin/spin

ENTRYPOINT [ "/usr/local/bin/spin" ]
