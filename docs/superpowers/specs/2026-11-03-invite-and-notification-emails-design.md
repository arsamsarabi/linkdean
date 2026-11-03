# Outbound email, part E: invite and notification emails

Design drafted on 2026-11-03 while the product owner was away, from the staff engineer's recommended defaults. Each decision marked **(default)** is open to change on review. Builds on parts A (outbox, templates), C (verified addresses, pending invites) and the in-app notifications in `crates/db/src/repo/notifications.rs`.

Everything here only happens while mail is on. With mail off the app behaves as today: in-app notifications, and claim links the owner passes on by hand.

## Invite emails

- Sharing a folder or dashboard with an address sends an email from the outbox.
  - **To an account that can receive the share** (verified, not blocking): `share_invite`: "{{ owner }} shared the folder "{{ name }}" with you ({{ access }})." Button "Open Linkdean" → `/shared`. The in-app notification is unchanged; the invite still needs accepting.
  - **To an address with no account, or an unverified account** (a pending invite): `share_invite_pending`: the same sentence plus "Create an account, or sign in, with this address to accept." Button "Accept the invite" → the claim link (`/register?share=…`), which also works while signed in (part C). The owner still sees the claim link to copy, as the fallback.
- No email when the invite is refused today (self-share, blocks), when the invitee declined an earlier invite from this owner (the existing "declined stops repeat invites" rule), or when only the access level of an existing share changes (that is a notification, below).
- The owner's name is their display name, else their address. Folder and dashboard names are escaped by the template engine; vault folders cannot be shared.
- **(default)** Rate limit: 20 invite emails per owner per hour, counted in the outbox, so a compromised account cannot use the instance to spam. Over the limit, the share is still created and the owner is told "The invite was saved, but no email was sent: too many invites in the last hour. Copy the link instead."

## Notification emails

- Settings → Notifications gets an "Email me" switch per kind. **(default)** values:

| Kind | Email by default |
| --- | --- |
| Share invites | On (this is the invite email above) |
| Share access changed, share revoked | Off |
| Edits in shared folders | Off |
| Likes, comments, ratings, new followers | Off |
| Moderation changes to your public content | On |

- An email is sent when the in-app notification is created, through the outbox, using the same rules that already decide who gets the notification (blocks, mutes, redacted vault titles).
- **(default)** No digest in this part: kinds that can be noisy (edits, social) are off by default. A daily digest goes to the backlog.
- Every notification email ends with "You got this because email for {{ kind }} is on. Turn it off in Settings → Notifications." with a link to that page.
- **(default)** One-click unsubscribe links (and the `List-Unsubscribe` header) go to the backlog; the settings link is the v1 way out.

## Data

- `user_notification_email_prefs (user_id, kind, enabled)`; a missing row means the default above.
- Templates: `share_invite`, `share_invite_pending`, `notification` (one generic template with a heading, one sentence, and a button to the right page; the sentence comes from the same text the in-app notification shows).

## Out of scope

- Digests, one-click unsubscribe, per-folder email settings.
- Organization invite emails (they come with organizations and teams, and will reuse `share_invite_pending`'s pattern).

## Testing

- Database: preference defaults and overrides; the invite rate count.
- API: invite email to a verified account, to a pending address, none on decline or block, none with mail off; the rate limit; a notification email sent only when its kind is on; muted and blocked users get none.
- Web: notification settings switches.
- Browser: share to a new address → Mailpit invite → claim link → register → invite visible; toggle a notification kind and see the email arrive.
