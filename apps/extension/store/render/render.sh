#!/usr/bin/env bash
# Renders the store screenshots (1280x800), the small promo tile (440x280) and the marquee
# tile (1400x560) into
# store/images, using the real popup build with sample data (stub.js) inside a drawn browser.
# Needs Google Chrome. Run from apps/extension: bash store/render/render.sh
set -euo pipefail
cd "$(dirname "$0")/../.."
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
PORT="${PORT:-8765}"
work="$(mktemp -d)"
server=""
cleanup() {
  if [ -n "$server" ]; then kill "$server" 2>/dev/null || true; fi
  rm -rf "$work"
}
trap cleanup EXIT

pnpm -s build >/dev/null
cp -R .output/chrome-mv3 "$work/ext"
cp store/render/stub.js "$work/ext/stub.js"
cp store/render/frame.html store/render/tile.html store/render/marquee.html "$work/"
cp ../../brand/final/linkdean-horizontal-white.svg "$work/"
cp public/icon/32.png "$work/icon32.png"
# Load the stub before the popup, and serve the popup's files from /ext.
sed -i.bak 's|<script type="module"|<script src="/ext/stub.js"></script><script type="module"|; s|src="/chunks|src="/ext/chunks|; s|href="/assets|href="/ext/assets|' "$work/ext/popup.html"

python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$work" >/dev/null 2>&1 &
server=$!
sleep 1

shot() {
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
    --blink-settings=preferredColorScheme=1 --user-data-dir="$work/profile" \
    --window-size="$1" --virtual-time-budget=6000 --screenshot="$2" "$3" >/dev/null 2>&1
}
for s in save saved signin menu; do
  shot 1280,800 "store/images/screenshot-$s.png" "http://127.0.0.1:$PORT/frame.html?s=$s"
done
shot 440,280 store/images/promo-small-440x280.png "http://127.0.0.1:$PORT/tile.html"
# The marquee crops the popup out of the save screenshot rendered above.
cp store/images/screenshot-save.png "$work/"
shot 1400,560 store/images/promo-marquee-1400x560.png "http://127.0.0.1:$PORT/marquee.html"
echo "Wrote store/images"
