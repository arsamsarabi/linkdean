#!/usr/bin/env bash
# Check on the guest that a deploy is live. Exits non-zero on the first failed check.
#
#   RELEASE_TAG=v0.2.0 CHECK_APP=true CHECK_DOCS=false CHECK_SITE=true bash verify.sh
#
# App: the api container runs the release image, is healthy, and /api/health reports the
# release version. Docs and site: Caddy serves the index.html that is checked out.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$ROOT"
STACK=deploy/public-beta
COMPOSE=(docker compose -f "$STACK/docker-compose.yml" --env-file "$STACK/.env")
TIMEOUT="${TIMEOUT:-300}"

fail() { echo "FAIL: $*" >&2; exit 1; }

# Fetch a page through Caddy from inside its container, with the public Host header.
caddy_get() {
  "${COMPOSE[@]}" exec -T caddy wget -qO- --header "Host: $1" "http://127.0.0.1$2"
}

if [ "${CHECK_APP:-false}" = true ]; then
  want="${RELEASE_TAG:?set RELEASE_TAG}"
  want="${want#v}"
  image="$("${COMPOSE[@]}" ps api --format '{{.Image}}')"
  case "$image" in
    *":$want") echo "api image: $image" ;;
    *) fail "api runs $image, expected tag $want" ;;
  esac

  deadline=$((SECONDS + TIMEOUT))
  until [ "$("${COMPOSE[@]}" ps api --format '{{.Health}}')" = healthy ]; do
    [ "$SECONDS" -lt "$deadline" ] || fail "api not healthy after ${TIMEOUT}s"
    sleep 5
  done
  echo "api: healthy"

  health="$(caddy_get app.linkdean.app /api/health)" || fail "app.linkdean.app/api/health unreachable through Caddy"
  got="$(printf '%s' "$health" | sed -n 's/.*"version":"\([^"]*\)".*/\1/p')"
  [ "$got" = "$want" ] || fail "/api/health reports version '$got', expected '$want'"
  echo "api: /api/health reports $got"

  caddy_get app.linkdean.app /api/ready >/dev/null || fail "/api/ready is not ready"
  echo "api: /api/ready ok"
fi

check_static() {
  local host=$1 dir=$2 want got
  want="$(sha256sum "$STACK/static/$dir/index.html" | cut -d' ' -f1)"
  got="$(caddy_get "$host" / | sha256sum | cut -d' ' -f1)" || fail "$host unreachable through Caddy"
  [ "$got" = "$want" ] || fail "$host serves an index.html that differs from static/$dir"
  echo "$host: serves the checked-out build"
}

if [ "${CHECK_DOCS:-false}" = true ]; then check_static docs.linkdean.app docs; fi
if [ "${CHECK_SITE:-false}" = true ]; then check_static linkdean.app site; fi

echo "verify: ok"
