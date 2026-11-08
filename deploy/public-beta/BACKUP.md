# Public beta backups

Off-site backups for the LXD guest `linkdean-public` on Nova. Design: [public-beta-restic-infisical](../../docs/superpowers/specs/2026-11-07-public-beta-restic-infisical-design.md).

## What is backed up

| Artifact | How |
| --- | --- |
| Postgres | `pg_dump -Fc` via Compose service `db` |
| Avatars + previews volumes | `tar` of Docker volumes `linkdean-public_avatars` and `linkdean-public_previews` |

Meilisearch is **not** dumped. After a restore, use **Settings → Admin → Reindex search**.

## Where it goes

- Tool: **restic** (same as the rest of the homelab)
- Bucket: Backblaze B2 `linkdean-public` (dedicated; one restic repo per bucket)
- Repo URL shape: `s3:s3.eu-central-003.backblazeb2.com/linkdean-public`
- Credentials + guest env SoT: Infisical `homelab` / `prod` / `/linkdean-public`
- Schedule on Nova: `0 * * * *` (hourly) → `~/novabot/restic/backup-linkdean.sh`
- Retention: 30 daily, 12 weekly, 12 monthly

## Env inject (deploy)

Before Compose restarts on a release, Nova runs:

```bash
bash deploy/public-beta/scripts/render-env-from-infisical.sh
```

That writes `/opt/linkdean/deploy/public-beta/.env` inside the guest from Infisical, **excluding** restic-only keys (`AWS_*`, `B2_*`, `RESTIC_*`).

## Manual backup

On Nova (with Infisical CLI or a restic dotenv):

```bash
/home/sudow/novabot/restic/backup-linkdean.sh
```

Reference copy in-repo: [`scripts/backup-linkdean.sh`](./scripts/backup-linkdean.sh).

## Restore drill

1. Pick a snapshot: `restic -r s3:$B2_ENDPOINT/$RESTIC_BUCKET_LINKDEAN snapshots --tag linkdean-public`
2. Restore into a work dir:  
   `restic -r … restore latest --target /tmp/linkdean-restore --tag linkdean-public`
3. On a **stopped** or empty guest stack (or a throwaway guest):  
   - `docker compose exec -T db pg_restore -U linkdean -d linkdean --clean --if-exists < linkdean.dump`  
     (or restore into a fresh DB volume)  
   - Extract `linkdean-files.tgz` onto the `avatars` / `previews` volumes (or `docker run` + `tar` into those volume names)  
   - Start the API; **Admin → Reindex search**
4. Confirm: sign-in, a known bookmark, an avatar or preview if one existed.

Do not run a destructive restore against the live public guest without an explicit go-ahead and a fresh backup taken first.
