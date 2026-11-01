# Share accept and reject

Status: agreed in chat, plan written, not implemented. Roadmap item: Sharing, "Accept / reject share flow".

## Goal

Sharing a folder or dashboard sends an invite. Access starts only when the invitee accepts it.

## Decisions

- Covers folders and dashboards.
- A row in `category_shares` or `dashboard_shares` still means access. Invites live in `pending_share_invites`, so an unaccepted share has no row in those tables and no read path needs a new filter. A forgotten check cannot leak a share.
- Existing shares stay active. Nobody is asked to re-accept on deploy.
- Changing the access level of an accepted share needs no new acceptance.
- Invites sent to an email with no account wait for the new user to accept. Signup links them to the account and no longer grants access.
- Rejecting is silent. The invite leaves the owner's pending list, and the owner gets no notification.
- A rejected invite blocks re-inviting that person to the same subject until the invitee restores it. The owner gets a 409: "This invite can't be sent." The message does not say why, but a blocked re-invite does tell a persistent owner something happened.
- The invitee restores a declined invite from Profile, which puts it back in their pending list. It does not grant access.
- Accepting sends no notification. The share moves to the owner's active list.
- Blocking in either direction revokes open invites, including rejected ones, so a later re-invite after unblocking works.

## Data model

Migration `028_share_invite_acceptance.sql` adds to `pending_share_invites`:

- `invitee_user_id UUID REFERENCES users(id) ON DELETE CASCADE`, because invites are keyed on email and an email can change. Accept, reject and restore all match on this column.
- `rejected_at TIMESTAMPTZ`.

It backfills `invitee_user_id` for open invites whose email matches an account. The unique index stays as it is: a rejected row is still open, which is what stops a duplicate invite.

An invite is open when `claimed_at` and `revoked_at` are null. It is pending when it is open and `rejected_at` is null, and declined when it is open and `rejected_at` is set.

## API

| Route | Result |
| --- | --- |
| `GET /share-invites` | The caller's open invites, with subject title, access, inviter name and `rejected_at` |
| `POST /share-invites/{id}/accept` | Creates the real share and closes the invite |
| `POST /share-invites/{id}/reject` | Sets `rejected_at` |
| `POST /share-invites/{id}/restore` | Clears `rejected_at` |

Each write returns 404 unless the invite belongs to the caller and is in the right state. Accepting when the subject is gone or a block exists revokes the invite and returns 404.

The existing create-share routes return the `pending` outcome for new invitees and `active` only when updating an existing share.

## Web

- The notifications drawer shows a section of pending invites with Accept and Decline above the notification list.
- The bell count adds pending invites, because opening the drawer marks notifications read and would otherwise hide an unanswered invite.
- Profile gets a "Declined invites" card with Restore.
- Share drawers say "Invite sent. They get access once they accept."

## Testing

- New DB tests: no access before accept, access after, reject and restore, re-invite blocked, other users cannot act on an invite, signup links without granting, email change, block revokes, deleted subject.
- Existing DB tests that share and then read go through accept.
- Web tests for the invite list.
