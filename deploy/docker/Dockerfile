# --- frontend ---
FROM node:22-alpine AS web
WORKDIR /web
COPY apps/web/package.json apps/web/pnpm-lock.yaml* ./
RUN corepack enable && (pnpm install --frozen-lockfile || pnpm install)
COPY apps/web/ ./
RUN pnpm build

# --- backend ---
FROM rust:1.89-bookworm AS api
WORKDIR /src
RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake libwebp-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates
COPY migrations ./migrations
# cache trick: empty main first? skip — full build
RUN cargo build --release -p linkdean-api

# --- runtime ---
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl libwebp7 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=api /src/target/release/linkdean-api /app/linkdean-api
COPY --from=web /web/dist /app/static
ENV STATIC_DIR=/app/static
# Named volumes take this ownership when they are first created.
RUN useradd --system --uid 10001 --no-create-home linkdean \
    && mkdir -p /app/data/avatars /app/data/previews \
    && chown -R linkdean:linkdean /app/data
USER linkdean
EXPOSE 8080
CMD ["/app/linkdean-api"]
