# Public beta monitoring

External uptime checks for `linkdean.app`. They must run outside the homelab: when Nova, the guest or the home connection is down, nothing on them can report it.

## Provider

Use a hosted uptime service with a free tier, keyword checks and push alerts to a phone app. [UptimeRobot](https://uptimerobot.com/) is the default choice: its free plan covers the four checks below at a 5-minute interval, with keyword monitors and alerts through its mobile app. [Better Stack Uptime](https://betterstack.com/uptime) is a fine alternative. Free tiers change, so check the current limits when signing up.

Install the provider's phone app and turn on its push notifications. Email alone is too easy to miss.

## Checks

| Name | URL | Type | Expect | Interval |
| --- | --- | --- | --- | --- |
| App ready | `https://app.linkdean.app/api/ready` | Keyword | `"status":"ready"` | 5 min |
| App version | `https://app.linkdean.app/api/health` | Keyword | `"status":"ok"` | 5 min |
| Marketing | `https://linkdean.app/` | Keyword | `Linkdean` | 5 min |
| Docs | `https://docs.linkdean.app/` | HTTP status | 200 | 5 min |

- **App ready** is the one that matters. `/api/ready` checks Postgres and Meilisearch and answers 503 with `"status":"degraded"` when either is down, so a keyword check catches a half-working API that still returns a page. It is never cached.
- **App version** confirms the API process answers at all, so a failed ready check with a passing version check points at the database or search rather than the API.
- The static sites are cached at Cloudflare, so their checks can stay green for a while after the guest goes down. They catch DNS, tunnel or Cloudflare configuration mistakes, not origin outages. App ready covers the origin.
- Alert after 2 failed checks in a row, so one dropped packet does not wake anyone.

## Cloudflare tunnel alerts

Cloudflare can also notify when the tunnel loses its connector: **Notifications → Add → Tunnel Health Alert** for the `linkdean-public` tunnel, sent to your email. It overlaps with App ready, but it names the cause (the guest's `cloudflared` is gone) rather than just the symptom.

## After setup

Stop the api container on the guest for 15 minutes (`docker compose stop api`, then `docker compose start api`), and check that the App ready alert reaches your phone and then clears. Do this in a quiet hour, since the app is down for everyone while it runs.
