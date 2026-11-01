# Profile and library visibility

Status: awaiting review. Roadmap item: Social, "Profile (own & others) + library visibility".

## Goal

Let a signed-in user view another user's profile and the bookmarks that user has chosen to show, and give every user control over who sees what. Nothing becomes visible to anyone as a result of deploying this feature.

## Terminology

"Public" means reachable without an account. It is reserved for the existing Public link (`/l/{slug}`) and does not change.

| UI label | Stored value | Audience |
| --- | --- | --- |
| Only me | `private` | The owner |
| Followers | `followers` | Users who follow the owner |
| Members | `members` | Anyone signed in on this instance |

A profile uses `hidden` in place of `private`, shown in the UI as "Hidden".

Help text next to every visibility control: "Members means anyone signed in here. Use a Public link for people without an account."

## Decisions

- Following stays instant and ungated. All followers see followers-tier content.
- Blocking is how a user removes a follower. It already drops follows and shares in both directions without notifying the other user.
- Muting does not affect profile access, because a mute only changes what the muter sees.
- Direct shares and Public links are explicit grants and keep working whatever the profile or category setting is.
- Vault bookmarks never appear on a profile.

## Approach

Add a separate read path: a new `profile_visible_bookmarks` query behind a new endpoint. `category_access` and `bookmark_can_read` keep their current meaning of explicit access through ownership or a share.

Two alternatives were rejected. Extending `category_access` would give search and social features access for free, but every Members category would enter every user's reachable set, and a wrong check could expose write paths. Materialized grants would need fan-out on every follow and block change, which this scale does not justify.

## Data model

Migration `026_profile_visibility.sql`:

| Column | Type | Default | Constraint |
| --- | --- | --- | --- |
| `users.profile_visibility` | `TEXT NOT NULL` | `'hidden'` | `IN ('members', 'followers', 'hidden')` |
| `categories.visibility` | `TEXT NOT NULL` | `'private'` | `IN ('private', 'followers', 'members')` |
| `bookmarks.is_private` | `BOOLEAN NOT NULL` | `FALSE` | |

Vault categories must stay `private`, enforced by a check constraint: `NOT is_vault OR visibility = 'private'`. The API returns 400 if a client tries to set anything else.

The defaults apply to new users as well as existing ones, so every user opts in.

Each category holds its own value. A child category does not inherit from its parent. `categories.is_hidden` is unrelated: it hides a category from its owner's own views and is not access control.

## Access rules

`profile_access(viewer, target)` returns allowed or not found:

1. Viewer is the target: allowed.
2. Blocked in either direction (`is_blocked_either_way`): not found.
3. `profile_visibility = 'members'`: allowed.
4. `profile_visibility = 'followers'` and the viewer follows the target: allowed.
5. Otherwise: not found.

Every denial is a 404, so a response never reveals whether a hidden user exists.

A bookmark appears on a profile when all of these hold:

- The profile is viewable under the rules above.
- The target owns the bookmark.
- `deleted_at IS NULL`, `is_vault` is false, `is_private` is false.
- It belongs to at least one category, through `bookmark_categories` or the legacy `bookmarks.category_id`, that allows the viewer: `members` for any viewer, `followers` when the viewer follows the target.

A bookmark with no category is not shown. When the viewer is the target, the same filter applies with both `followers` and `members` categories allowed, so the page previews everything other users could see.

## API

All routes require authentication.

| Route | Result |
| --- | --- |
| `GET /users/{id}/profile` | Avatar flag, display name, bio, follower and following counts, follow and block relation, `profile_visibility` for self only. `email` for self only. |
| `GET /users/{id}/bookmarks?limit=&offset=` | `{ bookmarks, total }` using the existing `PublicBookmarkView` shape, which has no notes or owner data. `limit` defaults to 50, maximum 100. Ordered by `updated_at` descending. |
| `GET /users/{id}/followers` | `{ users }` as a list of `ProfileSummary` (id, display name, avatar flag). Never includes email. |
| `GET /users/{id}/following` | Same shape as followers. |

All four use `profile_access`. Avatars of viewable users need a read route, `GET /users/{id}/avatar`, with the same gate.

Writes extend existing routes:

- `PATCH /users/me` accepts `profile_visibility`.
- The category update route accepts `visibility`.
- The bookmark update route accepts `is_private`.

Changes to `visibility` and `is_private` are written to the activity log. No notification is sent.

## Web

- New route `_app/u.$id.tsx`, so it sits behind login. A 404 renders a "profile not available" state with no further detail.
- `ProfileHeader` shows avatar, display name, bio, follower and following counts that open a list, and a follow button for other users.
- On the user's own profile: email and an edit button that opens an edit drawer with avatar, display name, bio, email, password, Save and Cancel. This replaces the current `ProfilePage` form.
- `ProfileBookmarksList` shows visible bookmarks with paging, on other users' profiles and as a preview on the user's own.
- Links to `/u/{id}` from owner names on shared bookmarks.
- Visibility selects on the profile edit drawer and the category drawers, and an "Only me" switch on the bookmark drawer. The category select is disabled for vault categories.

## V1 limits

- Profile bookmarks are read-only. Likes, comments, ratings and forks stay behind `bookmark_can_read`.
- Profile bookmarks are not in Meilisearch results.
- There is no user directory. Profiles are reached by link.

## Error handling

- Invalid `limit`, `offset` or visibility value: 400.
- Setting a vault category to anything but `private`: 400.
- Any failed `profile_access` check, including an unknown user id: 404.
- A follower who is later blocked or unfollows loses access on the next request, because access is computed per request.

## Testing

- `crates/db/tests/profile_visibility.rs`: a matrix of viewer (self, follower, member, blocked) by profile setting by category tier, plus `is_private`, vault, trashed, uncategorized, a bookmark in two categories with different tiers, and access lost after unfollow.
- Route tests: every denial is a 404, email appears only for self, and the follower lists omit email.
- A test confirms that deploying the migration leaves existing data invisible to other users.
- Web component tests for `ProfileHeader`, the unavailable state, and the disabled vault select.

## Documentation

On completion, move the roadmap item to `docs/FEATURES.md`.
