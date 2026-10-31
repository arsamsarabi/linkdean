# Self-host Linkdean

1. Copy `.env.example` to `.env`. Set `SESSION_SECRET` and `APP_URL`.
2. `docker compose -f deploy/docker/docker-compose.yml up -d --build`
3. On first visit, register (bootstrap admin).
4. Optional OIDC: set issuer, client id, and secret. Callback: `{APP_URL}/api/auth/oidc/callback`.
5. Avatars persist in the `avatars` volume (`AVATAR_DIR`); include with Postgres backups.

Secrets are injected via environment variables (or the operator's preferred secret store). No Infisical dependency.
