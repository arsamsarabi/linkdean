# Linkdean

Self-hostable bookmarks manager.

**Stack:** Rust (axum) · Postgres · Meilisearch · Vite/React/Mantine

## Features (v1)

- Nested categories + tags
- Meilisearch full-text (non-vault)
- Import: Netscape HTML, Linkdean JSON, Pocket CSV, Raindrop CSV
- Export: JSON + Netscape HTML
- Hidden categories
- Password-protected vault (Argon2id + AES-GCM)
- Local auth + optional OIDC
- Multi-device via server account

## Quick start (Compose)

```bash
cp .env.example .env
# edit SESSION_SECRET
cd deploy/docker && docker compose up -d --build
```

Open http://localhost:8080

## Dev

```bash
# infra
docker compose -f deploy/docker/docker-compose.yml up -d db meilisearch

cp .env.example .env
# DATABASE_URL=postgres://linkdean:linkdean@127.0.0.1:5432/linkdean

cargo run -p linkdean-api
cd apps/web && pnpm install && pnpm dev
```

## Env

See `.env.example`. OIDC optional (`OIDC_ISSUER`, `OIDC_CLIENT_ID`, `OIDC_CLIENT_SECRET`).

## Docs

- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Self-host](docs/self-host.md)

## License

MIT
