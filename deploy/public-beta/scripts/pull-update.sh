#!/usr/bin/env bash
# Roll the public-beta stack forward on the guest. Run from anywhere inside the repo.
#
# The Release workflow runs this over SSH after checking out the release tag:
#   RELEASE_TAG=v0.2.0 UPDATE_APP=true UPDATE_DOCS=false UPDATE_SITE=true bash pull-update.sh
#
# RELEASE_TAG (optional) pins the api image: LINKDEAN_VERSION in .env becomes the tag
# without its leading "v", so a later manual `docker compose up` keeps the same image.
# UPDATE_APP / UPDATE_DOCS / UPDATE_SITE pick what to refresh. If none is set, they are
# worked out from the files changed between REF_BEFORE (or ORIG_HEAD) and HEAD.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
STACK=deploy/public-beta
COMPOSE=(docker compose -f "$STACK/docker-compose.yml" --env-file "$STACK/.env")

REF_BEFORE="${REF_BEFORE:-}"
if [ -z "$REF_BEFORE" ] && git rev-parse -q --verify ORIG_HEAD >/dev/null; then
  REF_BEFORE="$(git rev-parse ORIG_HEAD)"
fi

changed() {
  [ -n "$REF_BEFORE" ] && git diff --name-only "$REF_BEFORE" HEAD | grep -Eq "$1"
}

UPDATE_APP="${UPDATE_APP:-}"
UPDATE_DOCS="${UPDATE_DOCS:-}"
UPDATE_SITE="${UPDATE_SITE:-}"

if [ -z "$UPDATE_APP$UPDATE_DOCS$UPDATE_SITE" ]; then
  UPDATE_APP=false UPDATE_DOCS=false UPDATE_SITE=false
  if changed '^(crates/|apps/web/|migrations/|deploy/docker/|Cargo\.(toml|lock)$)'; then UPDATE_APP=true; fi
  if changed '^(apps/docs/|deploy/public-beta/static/docs/|docs/(INSTALL|SELF_HOSTED|FEATURES|ARCHITECTURE)\.md$)'; then UPDATE_DOCS=true; fi
  if changed '^(apps/site/|deploy/public-beta/static/site/)'; then UPDATE_SITE=true; fi
fi

echo "update app=$UPDATE_APP docs=$UPDATE_DOCS site=$UPDATE_SITE tag=${RELEASE_TAG:-none}"

if [ "$UPDATE_APP" = true ]; then
  if [ -n "${RELEASE_TAG:-}" ]; then
    version="${RELEASE_TAG#v}"
    if grep -q '^LINKDEAN_VERSION=' "$STACK/.env"; then
      sed -i.bak "s/^LINKDEAN_VERSION=.*/LINKDEAN_VERSION=$version/" "$STACK/.env"
    else
      printf '\nLINKDEAN_VERSION=%s\n' "$version" >> "$STACK/.env"
    fi
    echo "LINKDEAN_VERSION=$version"
  fi
  "${COMPOSE[@]}" pull api
  "${COMPOSE[@]}" up -d api
fi

# Docs and site are bind-mounted from deploy/public-beta/static/, so the checkout already
# holds the new files. Restart Caddy so nothing stale stays cached in the container.
if [ "$UPDATE_DOCS" = true ] || [ "$UPDATE_SITE" = true ]; then
  "${COMPOSE[@]}" up -d caddy
  "${COMPOSE[@]}" restart caddy
fi

if [ "$UPDATE_APP" != true ] && [ "$UPDATE_DOCS" != true ] && [ "$UPDATE_SITE" != true ]; then
  echo "nothing to update"
fi
