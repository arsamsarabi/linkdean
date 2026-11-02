# Self-host Linkdean

This page covers the production stack in `deploy/docker/docker-compose.yml`. For local development, see [Development setup](#development-setup) below and the README.

## Quick start

```bash
git clone https://github.com/arsamsarabi/linkdean.git
cd linkdean
printf 'POSTGRES_PASSWORD=%s\nMEILI_MASTER_KEY=%s\nSESSION_SECRET=%s\n' \
  "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" "$(openssl rand -hex 32)" > deploy/docker/.env

docker compose --env-file deploy/docker/.env -f deploy/docker/docker-compose.yml up -d --build
```

Open `http://localhost:8080`. The first registered user becomes admin.

1. Create `deploy/docker/.env` with the [required secrets](#required-secrets) and set `APP_URL`. Behind a reverse proxy, also set `TRUST_FORWARDED_FOR=true` (see [Reverse proxy](#reverse-proxy)).
2. Compose up (command above).
3. On first visit, register (bootstrap admin).
4. Optional OIDC: set issuer, client id, and secret. `OIDC_ISSUER` is the provider's issuer exactly as its `/.well-known/openid-configuration` reports it, trailing slash included (Authentik: `https://auth.example.com/application/o/<slug>/`). Callback: `{APP_URL}/api/auth/oidc/callback`. OIDC sign-in obeys closed registration: an unknown identity gets no account unless registration is open.
5. Avatars persist in the `avatars` volume (`AVATAR_DIR`); screenshots + favicon/OG binaries in `previews` (`PREVIEW_DIR`). Include both with Postgres backups.

Secrets are injected via environment variables (or the operator's preferred secret store). No Infisical dependency. `deploy/docker/.env` is gitignored; keep it out of version control.

## Production deployment

### Required secrets

The production Compose file refuses to start unless all three are set:

| Variable | Used by | Notes |
| --- | --- | --- |
| `POSTGRES_PASSWORD` | `db`, and `api` through `DATABASE_URL` | Use hex (`openssl rand -hex 32`) so it stays valid inside the connection URL |
| `MEILI_MASTER_KEY` | `meilisearch`, `api` | Meilisearch runs with this key; the API uses it to index and search |
| `SESSION_SECRET` | `api` | Long random string |

Without one of them, `docker compose up` stops with `set POSTGRES_PASSWORD` (or the matching name).

The API also refuses to start when a secret is a value published in this repository: the `.env.example` and dev Compose defaults for `SESSION_SECRET` and `MEILI_MASTER_KEY`, and the `linkdean:linkdean` database login. Generate your own values. `ALLOW_DEFAULT_SECRETS=true` turns this check off; it exists for local development only and must never be set in production.

### Reverse proxy

Login, registration and vault-unlock limits count attempts per client address. By default the API uses the address of the TCP connection.

- **Behind a reverse proxy** (Caddy, nginx, Traefik, a tunnel): set `TRUST_FORWARDED_FOR=true`. Otherwise every request appears to come from the proxy, all users share one counter, and a few failed logins from anyone block logins for everyone.
- **API reachable directly**: leave `TRUST_FORWARDED_FOR` unset. With it on, a client can forge `X-Forwarded-For` and pick any address, which defeats the per-address limits.

Only enable it when the proxy is the sole route to the API, and make sure the proxy appends the client address to `X-Forwarded-For`. The API reads the last entry, the one your proxy wrote. With a chain of proxies, the last entry is the nearest proxy's view, so terminate the chain at a single proxy in front of the API.

### Container runtime

- The API image runs as an unprivileged user, `linkdean` (UID/GID 10001). It owns `/app/data/avatars` and `/app/data/previews`; the binary and static files stay read-only to it.
- Every service uses `restart: unless-stopped`, so the stack comes back after a crash or host reboot.
- Only `api` publishes a port (`8080`). Postgres, Meilisearch and Chrome stay on the Compose network.

### Limits and behaviour operators should know

| Area | Behaviour |
| --- | --- |
| Failed logins | 10 per address per account, 50 per address, 50 per account, each per 15 minutes. Past a limit the API returns `429` until the window ends. A successful login clears only the per-address-per-account counter |
| Account lockout | 50 failed logins on one account from any address lock that account for 15 minutes, **including the correct password**. Someone who knows a user's email can trigger this on purpose |
| Registration | 10 attempts per address per 15 minutes, counted whether or not they succeed |
| Vault unlock, recovery, global unlock | 10 failed attempts per user per 15 minutes, shared across all three |
| Limit storage | Counters live in API memory. Restarting the API resets them, and several API replicas would each keep their own |
| Not limited yet | Password change and global vault passphrase change |
| Password hashing | At most 4 Argon2 hashes run at once; extra logins queue |
| Expired session | The API returns `401` with `{"code":"session_expired"}`, and the web app signs the user out and opens `/login`. A wrong password stays a plain `401` |
| Database queries | Each statement times out after 30 seconds. Migrations run without this limit |
| Import | Request body up to 20 MiB; file content up to 10 MiB; folders nested at most 64 deep; at most 10,000 folders per import. Folder loops are refused |
| Avatar upload | Request body up to 6 MiB (larger uploads currently fail with `400`, not `413`) |
| Preview upload | Request body up to 9 MiB |
| Image decode (avatar, preview) | At most 40 megapixels and 160 MiB of decode memory per image; two decodes run at once and the rest wait |
| Trash retention | User setting from 1 to 3,650 days, enforced by a database `CHECK` |

### API clients

`PATCH` bodies distinguish a missing field from `null`:

- Field omitted → left unchanged.
- Field set to `null` → cleared. This applies to profile `display_name`, `bio` and `timezone`; bookmark `description`, `notes` and `category_id`; and category `parent_id` (moves the category to the root) and `icon`.

Scripts written against older builds that sent `null` to mean "no change" will now clear those fields.

### Upgrading an existing install

Earlier builds ran as root with a default database password. Before upgrading past commit `bdef6d8`:

1. **Database password.** Postgres sets its password only when the `db` volume is first created, so a new `POSTGRES_PASSWORD` does not change an existing database. If the database still uses `linkdean`, change it to match your new value before starting the new `api`:

   ```bash
   docker compose -f deploy/docker/docker-compose.yml exec db \
     psql -U linkdean -d linkdean -c "ALTER USER linkdean PASSWORD '<new POSTGRES_PASSWORD>'"
   ```

   The API refuses the old `linkdean:linkdean` login, so skipping this step stops it at startup.
2. **Meilisearch key.** A new `MEILI_MASTER_KEY` takes effect when Meilisearch restarts. If search is empty afterwards, run **Settings → Admin → Reindex search**.
3. **Volume ownership.** Files written by the old root container stay owned by root, and the new UID 10001 process cannot write them. Fix once:

   ```bash
   docker run --rm -v linkdean_avatars:/avatars -v linkdean_previews:/previews alpine \
     chown -R 10001:10001 /avatars /previews
   ```

4. **Migrations** run on API start. Migration `034` caps trash retention at 3,650 days and clamps larger values. Migration `035` removes decrypted vault titles from old activity entries.
5. **Reverse proxy.** Set `TRUST_FORWARDED_FOR=true` if you run behind one.

## Development setup

The dev stack (`deploy/docker/docker-compose.dev.yml`) and `.env.example` use well-known secrets and set `ALLOW_DEFAULT_SECRETS=true`, so they start without generating anything. Never copy those values into production.

The dev and production Compose files share a project name and volumes. Run one at a time. A database volume first created by the dev stack keeps the password `linkdean`; follow [Upgrading an existing install](#upgrading-an-existing-install) step 1 before switching that volume to production.

Running the API and web on the host is covered in the README under Development.

## Healthchecks

| Probe | Path | Meaning |
| --- | --- | --- |
| Liveness | `GET /api/health` | Process up |
| Readiness | `GET /api/ready` | Postgres + Meilisearch reachable |
| Metrics | `GET /api/metrics` | Prometheus scrape. Off unless `METRICS_TOKEN` is set; then send `Authorization: Bearer <token>` |

`/api/ready` returns `true` or `false` per check. Failure details go to the API log, not the response.

Metrics are labelled by route template (such as `/api/l/{slug}`), never by raw path.

Compose marks `db`, `meilisearch`, and `api` healthy before dependents start. Example:

```bash
curl -fsS http://localhost:8080/api/health
curl -fsS http://localhost:8080/api/ready
```

## Backup

Stop writes briefly (or accept a short inconsistency window), then dump Postgres + Meili + volumes.

```bash
# from deploy/docker (compose project name: linkdean)
set -a; . ./.env; set +a
COMPOSE="docker compose -f docker-compose.yml"

# 1. Postgres
$COMPOSE exec -T db pg_dump -U linkdean -d linkdean -Fc > "linkdean-pg-$(date +%F).dump"

# 2. Meilisearch dump (creates a dump task; file lands under meili volume)
$COMPOSE exec -T meilisearch \
  wget -qO- --header="Authorization: Bearer ${MEILI_MASTER_KEY:?}" \
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

Restore Postgres with `pg_restore` into a fresh DB, restore Meili dump per [Meilisearch dump docs](https://www.meilisearch.com/docs/learn/dump), then extract the files tarball back onto the named volumes and run the `chown` from [Upgrading](#upgrading-an-existing-install) step 3 if the tarball was made as root. After restore, use **Settings → Admin → Reindex search** if the Meili dump was skipped.
