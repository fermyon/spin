FROM fermyon/e2e-tests-toolchain

ARG BUILD_SPIN=false
ARG FETCH_SPIN=true
ARG SPIN_VERSION=canary

WORKDIR /root

RUN if [ "${FETCH_SPIN}" = "true" ]; then                                                                                       \
    wget https://github.com/fermyon/spin/releases/download/${SPIN_VERSION}/spin-${SPIN_VERSION}-linux-amd64.tar.gz &&           \
    tar -xvf spin-${SPIN_VERSION}-linux-amd64.tar.gz &&                                                                         \
    ls -ltr &&                                                                                                                  \
    mv spin /usr/local/bin/spin;                                                                                                \
    fi

WORKDIR /e2e-tests
COPY . .

RUN if [ "${BUILD_SPIN}" = "true" ]; then                                                                                       \
    cargo build --release &&                                                                                                    \
    cp target/release/spin /usr/local/bin/spin;                                                                                 \
    fi

RUN if [ "${FETCH_SPIN}" = "true" ] || [ "${BUILD_SPIN}" = "true" ]; then                                                       \
    spin --version;                                                                                                             \
    fi

CMD cargo test spinup_tests --features e2e-tests --no-fail-fast -- --nocapture
