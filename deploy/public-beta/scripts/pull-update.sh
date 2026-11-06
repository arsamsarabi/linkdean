#!/usr/bin/env bash
# Selective public-beta roll-forward on the LXD guest (run from repo root after git pull).
# Env (optional): UPDATE_APP=true|false UPDATE_DOCS=true|false UPDATE_SITE=true|false
# If none are set, detects changes between ORIG_HEAD and HEAD (or REF_BEFORE..HEAD).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
COMPOSE=(docker compose -f deploy/public-beta/docker-compose.yml)

REF_BEFORE="${REF_BEFORE:-}"
if [ -z "$REF_BEFORE" ] && git rev-parse -q --verify ORIG_HEAD >/dev/null; then
  REF_BEFORE="$(git rev-parse ORIG_HEAD)"
fi

detect() {
  local pattern=$1
  if [ -n "${REF_BEFORE:-}" ]; then
    git diff --name-only "$REF_BEFORE" HEAD | grep -Eq "$pattern"
  else
    return 1
  fi
}

UPDATE_APP="${UPDATE_APP:-}"
UPDATE_DOCS="${UPDATE_DOCS:-}"
UPDATE_SITE="${UPDATE_SITE:-}"

if [ -z "$UPDATE_APP$UPDATE_DOCS$UPDATE_SITE" ]; then
  UPDATE_APP=false
  UPDATE_DOCS=false
  UPDATE_SITE=false
  detect '^(crates/|apps/web/|migrations/|deploy/docker/|Cargo\.(toml|lock))' && UPDATE_APP=true
  detect '^(apps/docs/|docs/(INSTALL|SELF_HOSTED|FEATURES|ARCHITECTURE)\.md)' && UPDATE_DOCS=true
  detect '^(apps/site/|brand/)' && UPDATE_SITE=true
fi

echo "update app=$UPDATE_APP docs=$UPDATE_DOCS site=$UPDATE_SITE"

if [ "$UPDATE_APP" = true ]; then
  # Prefer LINKDEAN_VERSION from .env (use "main" for continuous deploy from Deploy main workflow).
  "${COMPOSE[@]}" pull api
  "${COMPOSE[@]}" up -d api
  "${COMPOSE[@]}" ps api
fi

# Docs and site are bind-mounted from deploy/public-beta/static/. A git pull is enough;
# bounce Caddy so any aggressive in-container cache is cleared.
if [ "$UPDATE_DOCS" = true ] || [ "$UPDATE_SITE" = true ]; then
  "${COMPOSE[@]}" restart caddy
  echo "static refreshed; caddy restarted"
fi

if [ "$UPDATE_APP" != true ] && [ "$UPDATE_DOCS" != true ] && [ "$UPDATE_SITE" != true ]; then
  echo "nothing to update"
fi
