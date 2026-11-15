#!/usr/bin/env bash
# Install a built static site on the public-beta guest, where Caddy serves it from
# deploy/public-beta/static/<site> (mounted read-only at /srv/<site>).
#
# The Release workflow builds the docs and marketing sites on GitHub, copies each changed
# site's tarball into the guest, then runs:
#   bash install-static.sh docs /tmp/docs.tgz v0.2.0
#
# The new files are unpacked next to the live ones and swapped in with two renames, so
# Caddy never serves a half-copied site. The release tag goes in <site>/.release for
# verify.sh. By hand: build apps/docs (build/) or apps/site (dist/), tar the folder's
# contents (`tar -C apps/docs/build -czf docs.tgz .`), copy it to the guest and run this.
set -euo pipefail

SITE="${1:?usage: install-static.sh docs|site <tarball> [release-tag]}"
TARBALL="${2:?usage: install-static.sh docs|site <tarball> [release-tag]}"
TAG="${3:-manual}"
case "$SITE" in docs | site) ;; *) echo "unknown site: $SITE" >&2; exit 2 ;; esac

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
STATIC="$ROOT/deploy/public-beta/static"
mkdir -p "$STATIC"

NEW="$STATIC/.$SITE.new"
OLD="$STATIC/.$SITE.old"
rm -rf "$NEW" "$OLD"
mkdir "$NEW"
tar -xzf "$TARBALL" -C "$NEW"
[ -f "$NEW/index.html" ] || { echo "$TARBALL has no index.html at its root" >&2; rm -rf "$NEW"; exit 1; }
printf '%s\n' "$TAG" > "$NEW/.release"
# Caddy runs as root in its container but reads only; keep the files world-readable.
chmod -R a+rX "$NEW"

if [ -d "$STATIC/$SITE" ]; then mv "$STATIC/$SITE" "$OLD"; fi
mv "$NEW" "$STATIC/$SITE"
rm -rf "$OLD"
echo "$SITE: installed $TAG ($(find "$STATIC/$SITE" -type f | wc -l | tr -d ' ') files)"
