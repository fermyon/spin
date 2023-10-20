ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

RUN --mount=type=bind,source=/,target=/polyfill /polyfill/polyfill.sh
