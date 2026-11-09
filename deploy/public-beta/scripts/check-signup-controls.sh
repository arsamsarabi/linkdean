#!/usr/bin/env bash
# Check that the public beta's sign-up controls are on. Creates no account: every request
# is one the API must refuse before it writes anything.
#
#   bash deploy/public-beta/scripts/check-signup-controls.sh
#   BASE_URL=http://localhost:8080 bash deploy/public-beta/scripts/check-signup-controls.sh
#
# Checks:
#   1. /api/auth/config publishes a Turnstile site key (both TURNSTILE_* keys are set; the
#      API refuses to start with only one).
#   2. A sign-up from a disposable domain is refused (BLOCK_DISPOSABLE_EMAILS=true).
#   3. A sign-up without a captcha token is refused (Turnstile is enforced).
# Reports, without failing: whether outbound mail is on. REQUIRE_VERIFIED_EMAIL only takes
# effect when it is.
#
# The disposable check runs before the captcha check in the API, so 2 refuses on the domain
# and 3 on the captcha. Neither consumes the per-address registration rate limit. 2 and 3
# are skipped when Turnstile is off, since nothing would then stop them creating an account.
set -euo pipefail

BASE_URL="${BASE_URL:-https://app.linkdean.app}"
BASE_URL="${BASE_URL%/}"
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"

# The API refuses a sign-up whose terms version is not the current one, before any other
# check, so send the one this checkout ships.
if [ -z "${TERMS_VERSION:-}" ]; then
  TERMS_VERSION="$(sed -n 's/^pub const TERMS_VERSION: &str = "\(.*\)";/\1/p' \
    "$ROOT/crates/core/src/legal/mod.rs")"
fi
[ -n "$TERMS_VERSION" ] || { echo "FAIL: could not read TERMS_VERSION; set it" >&2; exit 1; }

failed=0
pass() { echo "ok:   $*"; }
fail() { echo "FAIL: $*" >&2; failed=1; }

# POST a sign-up and print "<status> <body>".
register() {
  local email="$1" body status
  body="$(mktemp)"
  status="$(curl -sS -o "$body" -w '%{http_code}' -X POST "$BASE_URL/api/auth/register" \
    -H 'Content-Type: application/json' \
    --data "{\"email\":\"$email\",\"password\":\"signup-smoke-$(date +%s)-long\",\"terms_version\":\"$TERMS_VERSION\"}")"
  printf '%s %s\n' "$status" "$(cat "$body")"
  rm -f "$body"
}

config="$(curl -fsS "$BASE_URL/api/auth/config")" || {
  echo "FAIL: GET $BASE_URL/api/auth/config" >&2
  exit 1
}

if printf '%s' "$config" | grep -q '"turnstile_site_key":"[^"]'; then
  pass "Turnstile site key published"
  turnstile=true
else
  fail "no Turnstile site key: set TURNSTILE_SITE_KEY and TURNSTILE_SECRET_KEY"
  turnstile=false
fi

if printf '%s' "$config" | grep -q '"mail_enabled":true'; then
  pass "outbound mail on (REQUIRE_VERIFIED_EMAIL can take effect)"
else
  echo "note: outbound mail is off, so REQUIRE_VERIFIED_EMAIL has no effect until SMTP_* is set"
fi

# Without Turnstile either sign-up below could create an account, so both run only when the
# site key shows Turnstile is on. Then the captcha refuses whatever the domain check lets by.
if [ "$turnstile" = true ]; then
  out="$(register "signup-smoke-$(date +%s)@mailinator.com")"
  case "$out" in
    400*"lasting email"*) pass "disposable domain refused" ;;
    *) fail "disposable domain not refused (BLOCK_DISPOSABLE_EMAILS): $out" ;;
  esac

  out="$(register "signup-smoke-$(date +%s)@linkdean.app")"
  case "$out" in
    400*captcha*) pass "sign-up without a captcha token refused" ;;
    *) fail "sign-up without a captcha token not refused: $out" ;;
  esac
else
  echo "skip: disposable and captcha checks (they need Turnstile on, or they could sign up)"
fi

exit "$failed"
