# syntax=docker/dockerfile:1.19.0

ARG TARGET_TRIPLE=x86_64-unknown-linux-musl
ARG TARGET_CPU=x86-64-v2
ARG MERMAID_CLI_IMAGE=minlag/mermaid-cli:latest

###############################################################################
# Builder toolchain with cargo-chef + TypeScript. Mirrors run before apk.
###############################################################################
FROM rust:1.91-alpine3.20 AS builder-base
ARG TARGET_TRIPLE
ARG TARGET_CPU
ENV RUSTFLAGS="-C target-cpu=${TARGET_CPU}"
WORKDIR /work

# Avoid repeated copies of the full source tree when only scripts change.
COPY deploy/docker/setup-mirrors.sh /tmp/deploy/docker/setup-mirrors.sh
COPY deploy/mirrors /tmp/deploy/mirrors
RUN chmod +x /tmp/deploy/docker/setup-mirrors.sh

RUN --mount=type=cache,target=/var/cache/apk,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    set -eux; \
    /tmp/deploy/docker/setup-mirrors.sh; \
    apk update; \
    apk add --no-cache \
        bash \
        binutils \
        build-base \
        ca-certificates \
        clang \
        cmake \
        curl \
        git \
        libstdc++ \
        nodejs \
        npm \
        pkgconf \
        python3; \
    /tmp/deploy/docker/setup-mirrors.sh; \
    npm_registry_file=/etc/soffio/npm-registry; \
    npm_registry_default="https://registry.npmjs.org"; \
    npm_registry="${npm_registry_default}"; \
    if [ -r "${npm_registry_file}" ]; then \
        npm_registry="$(tr -d '\r\n' < "${npm_registry_file}")"; \
        if [ -z "${npm_registry}" ]; then npm_registry="${npm_registry_default}"; fi; \
    fi; \
    npm config set registry "${npm_registry}" --global; \
    npm install --global --unsafe-perm typescript@latest; \
    cargo install --locked cargo-chef; \
    rustup target add "${TARGET_TRIPLE}"; \
    rm -rf /root/.cache /tmp/deploy

###############################################################################
# Generate the cargo-chef recipe that captures dependency build graph.
###############################################################################
FROM builder-base AS planner
COPY Cargo.toml Cargo.lock build.rs ./
COPY src src
RUN cargo chef prepare --recipe-path recipe.json

###############################################################################
# Pre-build dependencies with cargo-chef to maximise layer reuse.
###############################################################################
FROM builder-base AS deps
ARG TARGET_TRIPLE
COPY --from=planner /work/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --target "${TARGET_TRIPLE}" --recipe-path recipe.json

###############################################################################
# Build the application binary.
###############################################################################
FROM builder-base AS builder
ARG TARGET_TRIPLE
COPY --from=deps /usr/local/cargo /usr/local/cargo
COPY --from=deps /work/target /work/target
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --target "${TARGET_TRIPLE}"
RUN strip "target/${TARGET_TRIPLE}/release/soffio"

###############################################################################
# Runtime image based on the official Mermaid CLI container.
###############################################################################
ARG MERMAID_CLI_IMAGE
FROM ${MERMAID_CLI_IMAGE} AS runtime
ARG TARGET_TRIPLE
ARG TARGET_CPU
ENV RUSTFLAGS="-C target-cpu=${TARGET_CPU}" \
    PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium-browser
WORKDIR /app

USER root

COPY deploy/docker/setup-mirrors.sh /tmp/deploy/docker/setup-mirrors.sh
COPY deploy/mirrors /tmp/deploy/mirrors
COPY deploy/docker/puppeteer.config.json /tmp/puppeteer.config.json
COPY deploy/docker/mmdc-wrapper.sh /tmp/mmdc-wrapper.sh
RUN chmod +x /tmp/deploy/docker/setup-mirrors.sh

RUN set -eux; \
    /tmp/deploy/docker/setup-mirrors.sh; \
    addgroup -S soffio 2>/dev/null || true; \
    adduser -S -G soffio soffio 2>/dev/null || true; \
    install -d -o soffio -g soffio /var/lib/soffio; \
    install -m 0644 /tmp/puppeteer.config.json /puppeteer-config.json; \
    install -m 0755 /tmp/mmdc-wrapper.sh /usr/local/bin/mmdc; \
    rm -rf /tmp/deploy /tmp/puppeteer.config.json /tmp/mmdc-wrapper.sh

COPY --from=builder /work/target/${TARGET_TRIPLE}/release/soffio /usr/local/bin/soffio
RUN chmod 0755 /usr/local/bin/soffio && chown soffio:soffio /usr/local/bin/soffio

COPY deploy/docker/entrypoint-su.sh /usr/local/bin/entrypoint-su.sh
RUN chmod +x /usr/local/bin/entrypoint-su.sh

ENTRYPOINT ["/usr/local/bin/entrypoint-su.sh"]
CMD ["/usr/local/bin/soffio"]
