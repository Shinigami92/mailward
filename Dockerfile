# syntax=docker/dockerfile:1

# ----- Stage 1: build the SPA -----
FROM node:22-bookworm-slim AS web
WORKDIR /app
RUN corepack enable
# Install deps first (cached unless the manifests change).
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY apps/web/package.json apps/web/package.json
RUN --mount=type=cache,target=/root/.local/share/pnpm/store \
    pnpm install --frozen-lockfile
# Build the static SPA into apps/web/dist.
COPY apps/web apps/web
RUN pnpm --filter @mailward/web build

# ----- Stage 2: build the API binary -----
FROM rust:1.95-bookworm AS api
WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
# Cache the cargo registry + target dir across builds; copy the binary out so it
# survives outside the (ephemeral) cache mount.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p mailward-api \
    && cp target/release/mailward-api /usr/local/bin/mailward-api

# ----- Stage 3: runtime -----
FROM debian:bookworm-slim AS runtime
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
