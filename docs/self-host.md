# Self-host Linkdean

## Quick start

```bash
git clone https://github.com/arsamsarabi/linkdean.git
cd linkdean
printf 'POSTGRES_PASSWORD=%s\nMEILI_MASTER_KEY=%s\nSESSION_SECRET=%s\n' \
  "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" > deploy/docker/.env

docker compose --env-file deploy/docker/.env -f deploy/docker/docker-compose.yml up -d --build
```

Open `http://localhost:8080`. First registered user → admin.

1. Create `deploy/docker/.env` with `POSTGRES_PASSWORD`, `MEILI_MASTER_KEY` and `SESSION_SECRET` from `openssl rand -hex 32`, and set `APP_URL`. Behind a reverse proxy, also set `TRUST_FORWARDED_FOR=true`.
2. Compose up (command above).
3. On first visit, register (bootstrap admin).
4. Optional OIDC: set issuer, client id, and secret. Callback: `{APP_URL}/api/auth/oidc/callback`.
5. Avatars persist in the `avatars` volume (`AVATAR_DIR`); screenshots + favicon/OG binaries in `previews` (`PREVIEW_DIR`). Include both with Postgres backups.

Secrets are injected via environment variables (or the operator's preferred secret store). No Infisical dependency.

## Healthchecks

| Probe | Path | Meaning |
| --- | --- | --- |
| Liveness | `GET /api/health` | Process up |
| Readiness | `GET /api/ready` | Postgres + Meilisearch reachable |

Compose marks `db`, `meilisearch`, and `api` healthy before dependents start. Example:

```bash
curl -fsS http://localhost:8080/api/health
curl -fsS http://localhost:8080/api/ready
```

## Backup

Stop writes briefly (or accept a short inconsistency window), then dump Postgres + Meili + volumes.

```bash
# from deploy/docker (compose project name: linkdean)
COMPOSE="docker compose -f docker-compose.yml"

# 1. Postgres
$COMPOSE exec -T db pg_dump -U linkdean -d linkdean -Fc > "linkdean-pg-$(date +%F).dump"

# 2. Meilisearch dump (creates a dump task; file lands under meili volume)
$COMPOSE exec -T meilisearch \
  wget -qO- --header="Authorization: Bearer ${MEILI_MASTER_KEY:-dev_master_key_change_me}" \
  --post-data='' http://127.0.0.1:7700/dumps
# After the dump task completes, copy from the meili volume, e.g.:
# docker run --rm -v linkdean_meili:/data -v "$PWD":/out alpine \
#   sh -c 'cp /data/dumps/*.dump /out/'

# 3. File volumes (avatars + previews)
docker run --rm \
  -v linkdean_avatars:/avatars \
  -v linkdean_previews:/previews \
  -v "$PWD":/out alpine \
  tar czf /out/linkdean-files-$(date +%F).tgz -C / avatars previews
```

Restore Postgres with `pg_restore` into a fresh DB, restore Meili dump per [Meilisearch dump docs](https://www.meilisearch.com/docs/learn/dump), then extract the files tarball back onto the named volumes. After restore, use **Settings → Admin → Reindex search** if the Meili dump was skipped.
