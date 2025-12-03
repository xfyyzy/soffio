# syntax=docker/dockerfile:1.19.0

ARG TARGET_TRIPLE=x86_64-unknown-linux-musl
ARG TARGET_CPU=x86-64-v2
ARG MERMAID_CLI_IMAGE=minlag/mermaid-cli:latest
ARG PREBUILT_DIR=/prebuilt

###############################################################################
# Runtime image based on the official Mermaid CLI container.
# Binaries must be provided via PREBUILT_DIR and will be copied directly.
###############################################################################
FROM ${MERMAID_CLI_IMAGE} AS runtime
ARG TARGET_TRIPLE
ARG TARGET_CPU
ARG PREBUILT_DIR
ENV PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium-browser
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

# Copy prebuilt binaries (must exist) and validate presence.
COPY ${PREBUILT_DIR}/soffio /usr/local/bin/soffio
COPY ${PREBUILT_DIR}/soffio-cli /usr/local/bin/soffio-cli
RUN set -eux; \
    test -x /usr/local/bin/soffio; \
    test -x /usr/local/bin/soffio-cli; \
    chmod 0755 /usr/local/bin/soffio /usr/local/bin/soffio-cli; \
    chown soffio:soffio /usr/local/bin/soffio /usr/local/bin/soffio-cli

COPY deploy/docker/entrypoint-su.sh /usr/local/bin/entrypoint-su.sh
RUN chmod +x /usr/local/bin/entrypoint-su.sh

ENTRYPOINT ["/usr/local/bin/entrypoint-su.sh"]
CMD ["/usr/local/bin/soffio"]
