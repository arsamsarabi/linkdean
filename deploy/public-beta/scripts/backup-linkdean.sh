#!/usr/bin/env bash
# Back up the public-beta Linkdean LXD guest (Postgres + avatars/previews) to B2 via restic.
#
# Install on Nova:
#   install -m 0755 deploy/public-beta/scripts/backup-linkdean.sh /home/sudow/novabot/restic/
# Cron:
#   0 * * * * /home/sudow/novabot/restic/backup-linkdean.sh >> /home/sudow/novabot/restic/logs/restic-linkdean.log 2>&1
#
# Credentials from Infisical /linkdean-public (preferred), else RESTIC_ENV_FILE, else restic/.env.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

LXC="${LXC:-linkdean-public}"
STACK_DIR="${STACK_DIR:-/opt/linkdean/deploy/public-beta}"
DB_SERVICE="${DB_SERVICE:-db}"
PROJECT_NAME="${PROJECT_NAME:-linkdean-public}"

INFISICAL_ENV="${INFISICAL_ENV:-prod}"
INFISICAL_PATH="${INFISICAL_PATH:-/linkdean-public}"
PROJECT_ID="${INFISICAL_PROJECT_ID:-}"
LAN_API_DEFAULT="${INFISICAL_LAN_URL:-http://192.168.0.120:8104}"
export PATH="${HOME}/.local/bin:${PATH}"

RESTIC_KEEP_DAILY="${RESTIC_KEEP_DAILY:-30}"
RESTIC_KEEP_WEEKLY="${RESTIC_KEEP_WEEKLY:-12}"
RESTIC_KEEP_MONTHLY="${RESTIC_KEEP_MONTHLY:-12}"

if [[ -f "$SCRIPT_DIR/lib/common.sh" ]]; then
  # shellcheck source=/dev/null
  source "$SCRIPT_DIR/lib/common.sh"
  RESTIC_ROOT="$SCRIPT_DIR"
else
  RESTIC_ROOT="${RESTIC_ROOT:-$SCRIPT_DIR}"
  log() { printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*"; }
  restic_bin() {
    if [[ -x "$RESTIC_ROOT/bin/restic" ]]; then printf '%s\n' "$RESTIC_ROOT/bin/restic"
    else command -v restic; fi
  }
  restic_cmd() { "$(restic_bin)" "$@"; }
  restic_repo_url() {
    local bucket="$1" endpoint="${B2_ENDPOINT#https://}"
    endpoint="${endpoint#http://}"; endpoint="${endpoint#s3://}"; endpoint="${endpoint%/}"
    printf 's3:%s/%s' "$endpoint" "$bucket"
  }
  forget_and_prune() {
    local repo="$1"
    log "Applying retention to $repo"
    restic_cmd -r "$repo" forget \
      --keep-daily "$RESTIC_KEEP_DAILY" \
      --keep-weekly "$RESTIC_KEEP_WEEKLY" \
      --keep-monthly "$RESTIC_KEEP_MONTHLY" \
      --prune
  }
  init_repo_if_needed() {
    local repo="$1" restic
    restic="$(restic_bin)"
    if timeout 30 "$restic" -r "$repo" cat config >/dev/null 2>&1; then
      log "Repository ready: $repo"; return 0
    fi
    log "Initializing repository: $repo"
    timeout 120 "$restic" -r "$repo" init
  }
fi

load_linkdean_restic_env() {
  local tmp args
  if [[ -n "${RESTIC_ENV_FILE:-}" && -f "$RESTIC_ENV_FILE" ]]; then
    set -a; # shellcheck source=/dev/null
    source "$RESTIC_ENV_FILE"; set +a
  elif command -v infisical >/dev/null 2>&1; then
    if [[ -z "${INFISICAL_API_URL:-}" ]]; then
      if getent hosts secrets.arsam.app >/dev/null 2>&1; then
        INFISICAL_API_URL="https://secrets.arsam.app"
      else
        INFISICAL_API_URL="$LAN_API_DEFAULT"
      fi
    fi
    export INFISICAL_API_URL INFISICAL_DISABLE_UPDATE_CHECK=true
    if [[ -z "${INFISICAL_TOKEN:-}" && -f "${HOME}/.config/infisical/machine.env" ]]; then
      set -a; # shellcheck disable=SC1091
      source "${HOME}/.config/infisical/machine.env"; set +a
      if ! getent hosts secrets.arsam.app >/dev/null 2>&1; then
        INFISICAL_API_URL="$LAN_API_DEFAULT"; export INFISICAL_API_URL
      fi
      if [[ -n "${INFISICAL_CLIENT_ID:-}" && -n "${INFISICAL_CLIENT_SECRET:-}" ]]; then
        INFISICAL_TOKEN="$(infisical login --method=universal-auth \
          --client-id="$INFISICAL_CLIENT_ID" \
          --client-secret="$INFISICAL_CLIENT_SECRET" \
          --domain="$INFISICAL_API_URL" --plain --silent)"
        export INFISICAL_TOKEN
      fi
      [[ -n "${INFISICAL_PROJECT_ID:-}" && -z "$PROJECT_ID" ]] && PROJECT_ID="$INFISICAL_PROJECT_ID"
    fi
    tmp="$(mktemp)"
    args=(export --path="$INFISICAL_PATH" --env="$INFISICAL_ENV" --format=dotenv --domain="$INFISICAL_API_URL")
    [[ -n "$PROJECT_ID" ]] && args+=(--projectId="$PROJECT_ID")
    infisical "${args[@]}" >"$tmp"
    set -a; # shellcheck source=/dev/null
    source "$tmp"; set +a
    rm -f "$tmp"
  elif [[ -f "$RESTIC_ROOT/.env" ]]; then
    set -a; # shellcheck source=/dev/null
    source "$RESTIC_ROOT/.env"; set +a
  else
    echo "Need Infisical CLI, RESTIC_ENV_FILE, or $RESTIC_ROOT/.env" >&2
    exit 1
  fi
  : "${AWS_ACCESS_KEY_ID:?missing AWS_ACCESS_KEY_ID}"
  : "${AWS_SECRET_ACCESS_KEY:?missing AWS_SECRET_ACCESS_KEY}"
  : "${B2_ENDPOINT:?missing B2_ENDPOINT}"
  : "${RESTIC_PASSWORD:?missing RESTIC_PASSWORD}"
  : "${RESTIC_BUCKET_LINKDEAN:?missing RESTIC_BUCKET_LINKDEAN}"
  export AWS_ACCESS_KEY_ID AWS_SECRET_ACCESS_KEY RESTIC_PASSWORD
}

# Retention for this repo. Each run backs up from a fresh mktemp dir, and restic groups
# snapshots by host and paths by default, so every snapshot was its own group and nothing
# was ever forgotten. Group by host and tag instead.
linkdean_forget() {
  local repo="$1"
  log "Applying retention to $repo"
  restic_cmd -r "$repo" forget \
    --tag linkdean-public \
    --group-by host,tags \
    --keep-daily "$RESTIC_KEEP_DAILY" \
    --keep-weekly "$RESTIC_KEEP_WEEKLY" \
    --keep-monthly "$RESTIC_KEEP_MONTHLY" \
    --prune
}

guest() {
  lxc exec "$LXC" -- "$@"
}

main() {
  load_linkdean_restic_env
  command -v lxc >/dev/null || { echo "lxc not found" >&2; exit 1; }
  command -v "$(restic_bin)" >/dev/null || { echo "restic not found" >&2; exit 1; }

  # `staging` stays global: the EXIT trap runs after main returns, when a local is gone,
  # and under `set -u` the trap then fails and leaves the dump behind in /tmp.
  local repo av pre
  repo="$(restic_repo_url "$RESTIC_BUCKET_LINKDEAN")"
  staging="$(mktemp -d "${TMPDIR:-/tmp}/linkdean-backup.XXXXXX")"
  av="${PROJECT_NAME}_avatars"
  pre="${PROJECT_NAME}_previews"
  trap 'rm -rf "$staging"' EXIT

  init_repo_if_needed "$repo"

  log "Dumping Postgres (${LXC} / ${DB_SERVICE})"
  guest bash -lc "cd '$STACK_DIR' && docker compose --env-file .env exec -T '$DB_SERVICE' \
    sh -c 'pg_dump -U \"\$POSTGRES_USER\" -d \"\$POSTGRES_DB\" -Fc'" \
    >"$staging/linkdean.dump"
  [[ -s "$staging/linkdean.dump" ]] || { echo "empty Postgres dump" >&2; exit 1; }

  log "Archiving volumes $av + $pre"
  guest docker run --rm \
    -v "${av}:/avatars:ro" \
    -v "${pre}:/previews:ro" \
    -v /tmp:/out \
    alpine:3.20 \
    tar czf /out/linkdean-files.tgz -C / avatars previews
  lxc file pull "${LXC}/tmp/linkdean-files.tgz" "$staging/linkdean-files.tgz"
  guest rm -f /tmp/linkdean-files.tgz
  [[ -s "$staging/linkdean-files.tgz" ]] || { echo "empty files archive" >&2; exit 1; }

  log "restic backup → $repo"
  restic_cmd -r "$repo" backup \
    "$staging/linkdean.dump" \
    "$staging/linkdean-files.tgz" \
    --tag linkdean-public \
    --host linkdean-public

  linkdean_forget "$repo"
  log "Linkdean public-beta backup complete"
}

# Sourced by restore-drill.sh for the credential loader; run directly, it backs up.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  main "$@"
fi
