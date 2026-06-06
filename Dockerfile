# syntax=docker/dockerfile:1@sha256:87999aa3d42bdc6bea60565083ee17e86d1f3339802f543c0d03998580f9cb89

# ----- Stage 1: build the SPA -----
FROM node:26.3.0-trixie-slim@sha256:aa27a5fbf5acb298116a38133794f080406c6f8dfe52e2e2836bb55dc7cae8f0 AS web
WORKDIR /app
# Node 26 no longer bundles corepack, so install the pinned pnpm directly.
# Keep this version in sync with the root package.json "packageManager" field.
RUN npm install -g pnpm@11.5.2
# Install deps first (cached unless the manifests change).
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY apps/web/package.json apps/web/package.json
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
# Build the static SPA into apps/web/dist.
COPY apps/web apps/web
RUN pnpm --filter @mailward/web build

# ----- Stage 2: build the API binary -----
FROM rust:1.96-trixie@sha256:fb328f0f58becb23ba1719940a2c94ece8b0b48afa837d05b79ef64bc1e18f6e AS api
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
# Cache the cargo registry + target dir across builds; copy the binary out so it
# survives outside the (ephemeral) cache mount.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --locked --release -p mailward-api \
    && cp target/release/mailward-api /usr/local/bin/mailward-api

# ----- Stage 3: runtime -----
FROM debian:trixie-slim@sha256:b6e2a152f22a40ff69d92cb397223c906017e1391a73c952b588e51af8883bf8 AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home mailward \
    && mkdir -p /config /cache \
    && chown -R mailward:mailward /config /cache
COPY --from=api /usr/local/bin/mailward-api /usr/local/bin/mailward-api
COPY --from=web /app/apps/web/dist /app/web
ENV MAILWARD_WEB_DIR=/app/web \
    MAILWARD_CONFIG_DIR=/config \
    MAILWARD_CACHE_DIR=/cache \
    MAILWARD_BIND=0.0.0.0:8080
EXPOSE 8080
USER mailward
# /config = the YAML files (the source of truth); /cache = OAuth refresh tokens.
VOLUME ["/config", "/cache"]
ENTRYPOINT ["/usr/local/bin/mailward-api"]
