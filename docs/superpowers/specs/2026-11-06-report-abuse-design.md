# Report abuse

Roadmap **#7**. Branch `feat/report-abuse` (stacked on `feat/legal-pages-beta` for terms contact wording).

## Goals

- Users (and optionally anonymous visitors) can **Report** public lists, profiles, and comments with a reason.
- Admins get a **report queue** with dismiss / hide content / warn / ban actions.
- Abuse and takedown **contact** appears in Privacy/Terms and on the report UI (platform setting or env).

## Report targets

| Target | Key | Where UI lives |
| --- | --- | --- |
| Public list | category `public_slug` or category id | Public list page |
| Profile | user id | Profile page |
| Comment | comment id | Comment menu / overflow |

## API

- `POST /reports` (auth required for v1 — ponytail; anonymous later): `{ target_type, target_id, reason, details? }`
  - `target_type`: `public_list` | `profile` | `comment`
  - Validate target exists and is in a reportable state (public list has slug; profile viewable; comment exists)
  - Rate-limit per user
- `GET /admin/reports?status=open` — list for admins
- `POST /admin/reports/{id}/resolve` — `{ action: dismiss|hide|warn|ban, note? }`
  - `dismiss`: mark resolved
  - `hide`: best-effort hide target (disable public link / set profile private / delete comment) using existing moderation helpers where possible
  - `warn`: record action; optional email to reported user if mail on
  - `ban`: create ban via existing `ban_user` (require reason)

## Data

Migration `049_abuse_reports.sql`:

- `abuse_reports`: id, reporter_user_id, target_type, target_id (text/uuid string), reason, details, status (`open`|`resolved`), resolution_action, resolution_note, resolved_by, created_at, resolved_at
- Platform: `abuse_contact_email TEXT` nullable on `platform_settings` (admin-editable)

## Web

- Report control on public list page, profile page, comment UI
- Modal: reason select + optional details
- Admin: Reports panel with open queue + resolve actions
- Privacy/Terms: abuse contact line using platform email when set, else “contact the instance operator”

## Out of scope

- Anonymous reports
- Marketing site page
- Automated ML triage
