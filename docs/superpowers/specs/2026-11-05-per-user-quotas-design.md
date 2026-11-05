# Per-user quotas

Approved design for roadmap item **Per-user quotas**. Admins set instance-wide defaults; nothing is limited unless configured. Call sites resolve limits through one function so membership plans can plug in later.

## Decisions

| Decision | Choice |
| --- | --- |
| Scope | Platform defaults only (no per-user overrides in v1) |
| Unset | `NULL` = unlimited |
| Enrich | One combined meter: page HTML + favicon/OG + screenshot |
| Enrich period | Calendar day UTC (`period_start` date); schema is period-keyed for week/month later |
| Import vs bookmark max | Refuse the whole import if `current + new > max` |
| Membership | Not implemented; `resolve_limits(user)` + usage table keep the door open |

## Limits (`platform_settings`)

| Column | Kind | Notes |
| --- | --- | --- |
| `max_bookmarks` | gauge | Live `COUNT` of non-deleted owned bookmarks |
| `max_stored_image_bytes` | gauge | Avatar file + preview/favicon/og files for owned bookmarks |
| `max_enrich_actions_per_day` | counter | `user_quota_usage` meter `enrich` |
| `max_import_bytes` | request | Clamped to hard 10 MiB |
| `max_upload_image_bytes` | request | Clamped to avatar/preview hard caps |

## Enforcement

- Bookmarks: before insert in `create_bookmark` (covers API, import, copy paths).
- Import: after parse, before apply — refuse if bookmark gauge would overflow; also enforce `max_import_bytes`.
- Uploads: avatar and bookmark preview handlers — enforce upload byte max and stored-image gauge.
- Enrich: before each network action in `enrich_bookmark` (page, favicon, OG, screenshot); fail closed / skip when over.

## API / UX

- `Error::QuotaExceeded { meter, limit, used }` → HTTP 403, JSON `code: "quota_exceeded"`.
- Admin: PATCH platform settings (null clears a limit).
- Users: `GET` usage endpoint; Settings shows used / limit when a limit is set.

## Out of v1

- Per-user override UI, Stripe/plans, enrich period UI other than day.
