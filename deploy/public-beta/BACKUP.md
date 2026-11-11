# Public beta backups

Off-site backups for the LXD guest `linkdean-public` on Nova. Operator notes: [managed-service.md](../../docs/ops/managed-service.md).

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
- Schedule on Nova: `0 */8 * * *` (00:00, 08:00, 16:00) → `~/novabot/restic/backup-linkdean.sh`
- Retention: the last 3 snapshots (one day at the schedule above), grouped by host and tag. Each run backs up from a fresh temp dir, so the default host-and-paths grouping would never prune

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

Run before announce, then after any change to the backup script or the stack's volumes. The drill restores a snapshot into a throwaway Compose project (`linkdean-restore`, from [`restore/docker-compose.yml`](./restore/docker-compose.yml)) inside the guest, checks it, and tears it down. The live project is only read: its `.env` for the image tag and its row counts for comparison. The drill stack's network is internal, so the restored API cannot send email or fetch pages.

On Nova, once:

```bash
install -m 0755 deploy/public-beta/scripts/restore-drill.sh /home/sudow/novabot/restic/
install -m 0755 deploy/public-beta/scripts/backup-linkdean.sh /home/sudow/novabot/restic/
install -D -m 0644 deploy/public-beta/restore/docker-compose.yml /home/sudow/novabot/restic/restore/docker-compose.yml
```

The drill sources `backup-linkdean.sh` for its credential loader, so both must be the versions from this repo. It pushes `restore/docker-compose.yml` into the guest itself, so the guest's checkout does not need it. Then:

```bash
/home/sudow/novabot/restic/restore-drill.sh            # latest snapshot
SNAPSHOT=1a2b3c4d /home/sudow/novabot/restic/restore-drill.sh
KEEP=1 /home/sudow/novabot/restic/restore-drill.sh     # leave it running to look around
```

It fails when the snapshot lacks the dump or files archive, `pg_restore` reports any error, the restored API never reports ready, or the restored database has no users. On success it prints the time taken, restored and live row counts (users, bookmarks, categories) and the number of avatar and preview files. Add a line to the log below.

With `KEEP=1`, tear down afterwards:

```bash
lxc exec linkdean-public -- bash -lc 'cd /tmp/linkdean-restore-drill/compose && \
  LINKDEAN_VERSION=x DRILL_SESSION_SECRET=x docker compose -p linkdean-restore --profile api down -v'
```

### Restoring for real

The same steps, against the live project, when the guest's data is lost. Do not run this against the live guest without an explicit go-ahead and a fresh backup taken first.

1. Pick a snapshot: `restic -r s3:$B2_ENDPOINT/$RESTIC_BUCKET_LINKDEAN snapshots --tag linkdean-public`
2. Restore it: `restic -r … restore <id> --target /tmp/linkdean-restore --tag linkdean-public`
3. Stop `api`, then restore into an empty database: `docker compose exec -T db pg_restore -U linkdean -d linkdean --no-owner --clean --if-exists < linkdean.dump`
4. Extract `linkdean-files.tgz` onto the volumes: `docker run --rm -v linkdean-public_avatars:/avatars -v linkdean-public_previews:/previews -v "$PWD:/in:ro" alpine:3.20 tar xzf /in/linkdean-files.tgz -C /`
5. Start `api`, then **Settings → Admin → Reindex search**.
6. Confirm: sign-in, a known bookmark, an avatar or preview if one existed.

### Drill log

| Date (UTC) | Snapshot | Image | Took | Users / bookmarks restored | Notes |
| --- | --- | --- | --- | --- | --- |
| 2026-11-09 13:47 | `fd8c959d` (13:00 hourly) | 0.0.2 | 20s | 1 / 116 (26 categories, 1 avatar, 362 previews), matching live | First drill. Found and fixed: the backup's temp dir (with a plaintext dump) leaked on every run, and retention never pruned (each snapshot was its own group) |
