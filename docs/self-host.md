# Self-host Linkdean

1. Copy `.env.example` → `.env`, set `SESSION_SECRET`, `APP_URL`.
2. `docker compose -f deploy/docker/docker-compose.yml up -d --build`
3. First visit → register (bootstrap).
4. Optional OIDC: set issuer/client id/secret; callback `{APP_URL}/api/auth/oidc/callback`.
5. Avatars persist in the `avatars` volume (`AVATAR_DIR`); back up with Postgres.

No Infisical required. Inject secrets however you like.
