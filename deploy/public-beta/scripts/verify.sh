#!/usr/bin/env bash
# Check on the guest that a deploy is live. Exits non-zero on the first failed check.
#
#   RELEASE_TAG=v0.2.0 CHECK_APP=true CHECK_DOCS=false CHECK_SITE=true bash verify.sh
#
# App: the api container runs the release image, is healthy, and /api/health reports the
# release version. With screenshots on, Chrome runs behind the egress proxy on internal
# networks only. Docs and site: Caddy serves the index.html that is checked out.
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

  # With screenshots on, Chrome must have no route out but the api's egress proxy.
  if grep -Eq "^BROWSER_WS_URL=['\"]?http://chrome:" "$STACK/.env"; then
    chrome="$("${COMPOSE[@]}" --profile chrome ps -q chrome)"
    [ -n "$chrome" ] || fail "BROWSER_WS_URL is set but chrome is not running"
    docker inspect -f '{{json .Config.Cmd}}' "$chrome" | grep -q -- '--proxy-server=http://api:3128' ||
      fail "chrome runs without --proxy-server; recreate it from the compose file"
    net="$(docker inspect -f '{{range $k, $v := .NetworkSettings.Networks}}{{$k}} {{end}}' "$chrome")"
    for n in $net; do
      [ "$(docker network inspect -f '{{.Internal}}' "$n")" = true ] ||
        fail "chrome is on network $n, which is not internal"
    done
    echo "chrome: behind the egress proxy, on internal network(s) $net"
  fi
fi

check_static() {
  local host=$1 dir=$2 want got
  want="$(sha256sum "$STACK/static/$dir/index.html" | cut -d' ' -f1)"
  got="$(caddy_get "$host" / | sha256sum | cut -d' ' -f1)" || fail "$host unreachable through Caddy"
  [ "$got" = "$want" ] || fail "$host serves an index.html that differs from static/$dir"
  echo "$host: serves the checked-out build"
}

if [ "${CHECK_DOCS:-false}" = true ]; then
  check_static docs.linkdean.app docs
  # A docs page must come back as its own pre-rendered file, not the root shell.
  want="$(sha256sum "$STACK/static/docs/docs/intro/index.html" | cut -d' ' -f1)"
  got="$(caddy_get docs.linkdean.app /docs/intro/ | sha256sum | cut -d' ' -f1)" || fail "docs /docs/intro/ unreachable"
  [ "$got" = "$want" ] || fail "docs.linkdean.app/docs/intro/ does not serve static/docs/docs/intro/index.html"
  echo "docs.linkdean.app: serves pre-rendered pages"
fi
if [ "${CHECK_SITE:-false}" = true ]; then check_static linkdean.app site; fi

echo "verify: ok"
