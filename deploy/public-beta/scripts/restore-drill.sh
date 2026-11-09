#!/usr/bin/env bash
# Restore drill: restore a restic snapshot of the public beta into a throwaway stack inside
# the guest, check it, time it, and tear it down. The live `linkdean-public` project is only
# read (its .env for the image tag, and row counts to compare against).
#
# Run on Nova, next to backup-linkdean.sh (it reuses that script's credential loader):
#   install -m 0755 deploy/public-beta/scripts/restore-drill.sh /home/sudow/novabot/restic/
#   /home/sudow/novabot/restic/restore-drill.sh
#
# Env:
#   SNAPSHOT=latest   restic snapshot id to restore
#   KEEP=1            leave the drill stack running for a look around; tear down later with
#                     lxc exec linkdean-public -- docker compose -p linkdean-restore down -v
#
# Fails when: the snapshot lacks the dump or files archive, pg_restore errors, the restored
# API does not report ready, or the restored database has no users.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Credential loader, restic helpers and `guest`. Sourcing does not run a backup.
# shellcheck source=deploy/public-beta/scripts/backup-linkdean.sh
source "$SCRIPT_DIR/backup-linkdean.sh"

SNAPSHOT="${SNAPSHOT:-latest}"
KEEP="${KEEP:-0}"
DRILL_PROJECT=linkdean-restore
DRILL_DIR="$STACK_DIR/restore"
GUEST_TMP=/tmp/linkdean-restore-drill

started=$(date +%s)
elapsed() { echo $(($(date +%s) - started)); }

drill() {
  guest bash -lc "cd '$DRILL_DIR' && LINKDEAN_VERSION='$version' DRILL_SESSION_SECRET='$secret' \
    docker compose -p '$DRILL_PROJECT' $*"
}

# Row counts as "users bookmarks categories", from the given compose project's db.
counts() {
  local project_dir="$1" project="$2"
  guest bash -lc "cd '$project_dir' && LINKDEAN_VERSION='$version' DRILL_SESSION_SECRET='$secret' \
    docker compose -p '$project' exec -T db psql -U linkdean -d linkdean -Atc \
    \"SELECT (SELECT count(*) FROM users) || ' ' || (SELECT count(*) FROM bookmarks) || ' ' || (SELECT count(*) FROM categories)\""
}

teardown() {
  if [[ "$KEEP" == 1 ]]; then
    log "KEEP=1: drill stack left running (project $DRILL_PROJECT)"
  else
    drill "--profile api down -v --remove-orphans" >/dev/null 2>&1 || true
  fi
  guest rm -rf "$GUEST_TMP" || true
  rm -rf "${staging:-}"
}

main_drill() {
  load_linkdean_restic_env
  command -v lxc >/dev/null || { echo "lxc not found" >&2; exit 1; }

  local repo snap_dir dump files live restored
  repo="$(restic_repo_url "$RESTIC_BUCKET_LINKDEAN")"
  staging="$(mktemp -d "${TMPDIR:-/tmp}/linkdean-restore.XXXXXX")"
  secret="$(openssl rand -hex 32)"
  version="$(guest sed -n 's/^LINKDEAN_VERSION=//p' "$STACK_DIR/.env" | tail -1)"
  version="${version:-latest}"
  trap teardown EXIT

  log "Snapshot $SNAPSHOT from $repo"
  restic_cmd -r "$repo" snapshots --tag linkdean-public --latest 3
  restic_cmd -r "$repo" restore "$SNAPSHOT" --tag linkdean-public --target "$staging"
  # The backup stores files under their staging path, so find them wherever they landed.
  dump="$(find "$staging" -name linkdean.dump -type f | head -1)"
  files="$(find "$staging" -name linkdean-files.tgz -type f | head -1)"
  [[ -s "$dump" ]] || { echo "FAIL: snapshot has no linkdean.dump" >&2; exit 1; }
  [[ -s "$files" ]] || { echo "FAIL: snapshot has no linkdean-files.tgz" >&2; exit 1; }
  snap_dir="$(dirname "$dump")"
  log "Restored to $snap_dir after $(elapsed)s: dump $(du -h "$dump" | cut -f1), files $(du -h "$files" | cut -f1)"

  guest rm -rf "$GUEST_TMP"
  guest mkdir -p "$GUEST_TMP"
  lxc file push "$dump" "$LXC$GUEST_TMP/linkdean.dump"
  lxc file push "$files" "$LXC$GUEST_TMP/linkdean-files.tgz"

  log "Starting drill db + meilisearch (image tag $version)"
  drill "--profile api down -v --remove-orphans" >/dev/null 2>&1 || true
  drill "up -d --wait db meilisearch"

  log "pg_restore"
  guest bash -lc "cd '$DRILL_DIR' && LINKDEAN_VERSION='$version' DRILL_SESSION_SECRET='$secret' \
    docker compose -p '$DRILL_PROJECT' exec -T db \
    pg_restore -U linkdean -d linkdean --no-owner --exit-on-error < '$GUEST_TMP/linkdean.dump'"

  log "Extracting avatars + previews"
  guest docker run --rm \
    -v "${DRILL_PROJECT}_avatars:/avatars" \
    -v "${DRILL_PROJECT}_previews:/previews" \
    -v "$GUEST_TMP:/in:ro" \
    alpine:3.20 \
    tar xzf /in/linkdean-files.tgz -C /

  log "Starting the restored API"
  drill "--profile api up -d --wait api"
  guest bash -lc "cd '$DRILL_DIR' && LINKDEAN_VERSION='$version' DRILL_SESSION_SECRET='$secret' \
    docker compose -p '$DRILL_PROJECT' exec -T api curl -fsS http://127.0.0.1:8080/api/ready"
  echo

  restored="$(counts "$DRILL_DIR" "$DRILL_PROJECT")"
  live="$(counts "$STACK_DIR" "$PROJECT_NAME")"
  local av pre
  av="$(guest docker run --rm -v "${DRILL_PROJECT}_avatars:/d:ro" alpine:3.20 sh -c 'find /d -type f | wc -l')"
  pre="$(guest docker run --rm -v "${DRILL_PROJECT}_previews:/d:ro" alpine:3.20 sh -c 'find /d -type f | wc -l')"

  read -r r_users r_bookmarks r_categories <<<"$restored"
  read -r l_users l_bookmarks l_categories <<<"$live"
  [[ "${r_users:-0}" -gt 0 ]] || { echo "FAIL: restored database has no users" >&2; exit 1; }

  cat <<EOF

Restore drill passed in $(elapsed)s ($(date -u '+%Y-%m-%d %H:%M UTC'), snapshot $SNAPSHOT, image $version)
              restored   live now
  users       $r_users          $l_users
  bookmarks   $r_bookmarks          $l_bookmarks
  categories  $r_categories          $l_categories
  avatar files   $av
  preview files  $pre
Restored counts trail the live ones by whatever changed since the snapshot.
Add a line to BACKUP.md → Drill log.
EOF
}

main_drill "$@"
