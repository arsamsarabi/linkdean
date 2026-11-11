# Public beta monitoring

Uptime checks and page-view statistics for `linkdean.app`.

## Uptime: Uptime Kuma

The checks run in the operator's Uptime Kuma, on a homelab machine separate from Nova, in a monitor group named **Linkdean**. Alerts go to the phone through the existing ntfy notification. Each check alerts after 2 failures in a row, so one dropped packet does not wake anyone.

| Name | URL | Type | Expect | Interval |
| --- | --- | --- | --- | --- |
| Linkdean · App ready | `https://app.linkdean.app/api/ready` | Keyword | `"status":"ready"` | 1 min |
| Linkdean · App version | `https://app.linkdean.app/api/health` | Keyword | `"status":"ok"` | 1 min |
| Linkdean · Marketing | `https://linkdean.app/` | Keyword | `Linkdean` | 5 min |
| Linkdean · Docs | `https://docs.linkdean.app/` | HTTP status | 200–299 | 5 min |
| Linkdean · Outside heartbeat | the healthchecks.io ping URL | HTTP status | 200–299 | 5 min |

- **App ready** is the one that matters. `/api/ready` checks Postgres and Meilisearch and answers 503 with `"status":"degraded"` when either is down, so a keyword check catches a half-working API that still returns a page. It is never cached.
- **App version** confirms the API process answers at all, so a failed ready check with a passing version check points at the database or search rather than the API.
- The static sites are cached at Cloudflare, so their checks can stay green for a while after the guest goes down. They catch DNS, tunnel or Cloudflare configuration mistakes, not origin outages. App ready covers the origin.
- Every check goes out through the public internet and Cloudflare, the same way visitors arrive, so a broken tunnel or DNS record fails them too.

## The blind spot: the home connection

Uptime Kuma shares the home connection and power with Nova. When either drops, Kuma goes down with the app and cannot alert, and ntfy (also in the homelab) cannot deliver.

An outside heartbeat covers this. A free [healthchecks.io](https://healthchecks.io/) check expects a ping every 5 minutes, and **Linkdean · Outside heartbeat** in Kuma sends that ping by requesting the check's ping URL. When the pings stop, healthchecks.io alerts on its own.

- Check settings: period 5 minutes, grace 10 minutes.
- Its alerts must not go through the homelab: use email plus a phone channel that healthchecks.io delivers itself (its Telegram, Signal or Pushover integrations), not ntfy.
- An alert from healthchecks.io with no alert from Kuma means the whole homelab is unreachable. An alert from Kuma means the homelab is up and Linkdean is not.

## Cloudflare tunnel alerts

Cloudflare can also notify when the tunnel loses its connector: **Notifications → Add → Tunnel Health Alert** for the `linkdean-public` tunnel, sent to your email. It overlaps with App ready, but it names the cause (the guest's `cloudflared` is gone) rather than just the symptom.

## After setup

Stop the api container on the guest for 15 minutes (`docker compose stop api`, then `docker compose start api`), and check that the App ready alert reaches your phone and then clears. Do this in a quiet hour, since the app is down for everyone while it runs.

## Page views: GoatCounter

Page views are counted on the operator's self-hosted [GoatCounter](https://www.goatcounter.com/), as a site of its own at `https://stats.linkdean.app`.

- **Where it runs:** the GoatCounter container on Nova, as site 2, linked to the operator's existing site so it shares the same login. The homelab Traefik routes `stats.linkdean.app` to it, and the homelab Cloudflare tunnel publishes that hostname.
- **Dashboard:** set to private, so it asks for the GoatCounter login. The count endpoints (`/count`, `/count.js`) stay public for the trackers.
- **Marketing site and docs** load `count.js` from `stats.linkdean.app`. Paths are prefixed with the host (`docs.linkdean.app/self-hosting/quick-start`) so the three sites stay apart in one dashboard.
- **The app** counts only when the API has `GOATCOUNTER_URL=https://stats.linkdean.app` in the guest's `.env`. It sends one beacon per route with the route's name (`app.linkdean.app/bookmarks/$id`), never ids, search terms, titles or the referrer, and loads no script from GoatCounter. The CSP allows beacons to that origin only.
- Browsers that send Do Not Track or Global Privacy Control are not counted on any of the three sites.
- No cookies and nothing in browser storage, so no consent banner. The Privacy Policy, section 2a, describes what is sent.
