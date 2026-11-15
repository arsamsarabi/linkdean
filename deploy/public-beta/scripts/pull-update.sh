#!/usr/bin/env bash
# Roll the public-beta stack forward on the guest. Run from anywhere inside the repo.
#
# The Release workflow runs this over SSH after checking out the release tag:
#   RELEASE_TAG=v0.2.0 UPDATE_APP=true UPDATE_DOCS=false UPDATE_SITE=true bash pull-update.sh
#
# RELEASE_TAG (optional) pins the api image: LINKDEAN_VERSION in .env becomes the tag
# without its leading "v", so a later manual `docker compose up` keeps the same image.
# The docs and site files are not handled here: the Release workflow installs them with
# install-static.sh before this runs. This only recreates Caddy for them.
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
  if changed '^(apps/docs/|deploy/public-beta/Caddyfile$|deploy/public-beta/scripts/install-static\.sh$|docs/)'; then UPDATE_DOCS=true; fi
  if changed '^(apps/site/|deploy/public-beta/scripts/install-static\.sh$)'; then UPDATE_SITE=true; fi
fi

# Screenshots are on when .env points the api at the chrome service.
chrome_on() {
  grep -Eq "^BROWSER_WS_URL=['\"]?http://chrome:" "$STACK/.env"
}

# Compose never changes an existing network's options, so a `browser` network made before
# it was marked `internal` keeps a route out for Chrome. Remove it, and the two containers
# on it, so `up` makes it again from the compose file. The api is down for a few seconds.
ensure_browser_network() {
  local net
  net="$("${COMPOSE[@]}" config --format json | python3 -c 'import json,sys; print(json.load(sys.stdin)["networks"]["browser"]["name"])')"
  docker network inspect "$net" >/dev/null 2>&1 || return 0
  [ "$(docker network inspect -f '{{.Internal}}' "$net")" = true ] && return 0
  echo "$net is not internal; recreating it (api and chrome restart)"
  "${COMPOSE[@]}" --profile chrome rm -sf api chrome
  docker network rm "$net"
}

echo "update app=$UPDATE_APP docs=$UPDATE_DOCS site=$UPDATE_SITE tag=${RELEASE_TAG:-none}"

# Set KEY=value in .env, adding the line when it is missing.
pin() {
  if grep -q "^$1=" "$STACK/.env"; then
    sed -i.bak "s/^$1=.*/$1=$2/" "$STACK/.env"
  else
    printf '\n%s=%s\n' "$1" "$2" >> "$STACK/.env"
  fi
  echo "$1=$2"
}

if [ "$UPDATE_APP" = true ]; then
  if [ -n "${RELEASE_TAG:-}" ]; then pin LINKDEAN_VERSION "${RELEASE_TAG#v}"; fi
  if chrome_on; then
    ensure_browser_network
    "${COMPOSE[@]}" --profile chrome pull api chrome
    # Chrome too: its command and network change with the stack, not only with the image.
    "${COMPOSE[@]}" --profile chrome up -d api chrome
  else
    "${COMPOSE[@]}" pull api
    "${COMPOSE[@]}" up -d api
  fi
fi

# Docs and site: Caddy serves the files install-static.sh just swapped in, so they are live
# already. Recreate Caddy when its compose settings changed (`up -d`), and restart it so a
# changed Caddyfile is read. A guest still on the old linkdean-static image loses its
# LINKDEAN_STATIC_VERSION pin, which nothing reads any more.
if [ "$UPDATE_DOCS" = true ] || [ "$UPDATE_SITE" = true ]; then
  if grep -q '^LINKDEAN_STATIC_VERSION=' "$STACK/.env"; then sed -i.bak '/^LINKDEAN_STATIC_VERSION=/d' "$STACK/.env"; fi
  "${COMPOSE[@]}" pull caddy
  "${COMPOSE[@]}" up -d caddy
  "${COMPOSE[@]}" restart caddy
fi

if [ "$UPDATE_APP" != true ] && [ "$UPDATE_DOCS" != true ] && [ "$UPDATE_SITE" != true ]; then
  echo "nothing to update"
fi
