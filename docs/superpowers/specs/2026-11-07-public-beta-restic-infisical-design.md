# Public beta: Infisical env + restic/B2 backups

**Status:** accepted for implementation  
**Scope:** LXD guest `linkdean-public` on Nova only. Personal `linkdean.arsam.app` is out of scope.

## Goal

1. Infisical (`homelab` / `prod` / `/linkdean-public`) is the sole source of truth for the guest Compose `.env` and for restic/Backblaze credentials.
2. Off-site encrypted backups of Postgres + avatars/previews volumes via **restic** to a dedicated B2 bucket `linkdean-public`.
3. Secrets are injected at **deploy/runtime** on Nova, never baked into GHCR or static site builds.

## Architecture

```text
Infisical /linkdean-public
        │ export at deploy / backup
        ▼
Nova host (restic scripts + release runner)
        │ lxc file push / lxc exec
        ▼
LXD guest linkdean-public  ── Compose stack ──► app.linkdean.app
        │
        │ pg_dump + volume tar (via lxc exec)
        ▼
restic → s3:s3.eu-central-003.backblazeb2.com/linkdean-public
```

## Infisical

- Project `homelab`, environment `prod`, path `/linkdean-public`.
- Holds the full guest env (see [INSTALL.md](../../INSTALL.md) + [deploy/public-beta/.env.example](../../../deploy/public-beta/.env.example)) plus restic keys: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `B2_ENDPOINT`, `RESTIC_PASSWORD`, `RESTIC_BUCKET_LINKDEAN`.
- GitHub stores **no** app secrets (only `GITHUB_TOKEN` and repo variable `NOVA_DEPLOY_ENABLED`).
- Render filters restic-only keys out of the Compose `.env` written into the guest.

## Deploy inject

- Script: [`deploy/public-beta/scripts/render-env-from-infisical.sh`](../../../deploy/public-beta/scripts/render-env-from-infisical.sh) (runs on Nova).
- Release workflow calls it **before** `pull-update.sh` on the self-hosted `linkdean-nova` runner.
- `LINKDEAN_VERSION` may still be rewritten by `pull-update.sh` after render.

## Backups

- Host job: `/home/sudow/novabot/restic/backup-linkdean.sh` (reference copy in `deploy/public-beta/scripts/backup-linkdean.sh`).
- Schedule: `0 * * * *` (hourly on Nova).
- Contents: `pg_dump -Fc` of Compose `db`; tar of `avatars` + `previews` volumes. No Meili dump (reindex after restore).
- Retention: keep-daily 30, keep-weekly 12, keep-monthly 12.
- Operator docs: [`deploy/public-beta/BACKUP.md`](../../../deploy/public-beta/BACKUP.md).

## Non-goals

- Baking Infisical into CI image/site builds.
- Migrating or removing the personal Linkdean instance.
- Meili dump in the backup set.
