# Roadmap

What is still planned. Shipped capabilities live in [FEATURES.md](./FEATURES.md). Scratch ideas: [IDEAS.md](./IDEAS.md).

---

## Backlog 

**Social**

- _(likes, comments, ratings on shared items — already shipped)_

**Teams**

- Create team; invite / remove members
- View team, members, bookmarks/lists, dashboards
- Team activity, comments, and likes surfaces

**Vault & privacy**

- Client-held vault key option (true E2E; harder search tradeoff)

**Auth & multi-user**

- Passkeys / WebAuthn
- Configurable SSO providers: Authentik, Google, Microsoft, GitHub, etc.

**Platform**

- SMTP / outbound email config (for invites + notification delivery) ([react-email](https://www.npmjs.com/package/react-email) for templates)
  - Verify email addresses at registration and on email change, before the address can receive share invites. H1 in [POSSIBLE_DEFECTS.md](./POSSIBLE_DEFECTS.md).
  - Until an address is verified, invites to it wait as pending invites for the address rather than going to the unverified account.
- Localization / i18n-ready strings (and eventual locale packs)
- Mobile-friendly layout pass
- backend app to write logs for the app. in a structured and standard format. all activity and errors should be logged to file.
- Page the library list. `GET /bookmarks` returns every bookmark, and the web app filters and sorts the whole set in the browser (`useLibraryList` → `filterAndSortLibraryItems`). Large libraries pay for that on every load. The rest of M31 in [POSSIBLE_DEFECTS.md](./POSSIBLE_DEFECTS.md).
  - Use infinite scrolling, not numbered pages. The library is browsed and filtered rather than jumped through by page, and filters change the result set often.
  - Keyset pagination in the API (`cursor` + `limit`, ordered by the active sort key and `id`), so new or edited bookmarks do not shift or repeat rows the way `OFFSET` would.
  - Move folder, tag, flag, "since" and sort filters into the query, and return the total match count for headers and "select all".
  - Explorer loads one folder's contents at a time, rather than slicing the full list.
  - Bulk actions on "select all" act on the server-side match (filters sent to the API), not only the rows loaded so far. Keep H21's rule: select all reaches only what the open folder or search shows.
  - Vault search stays a Postgres scan of unlocked rows. Page it the same way.
  - Batch the access checks for bookmarks shared with the viewer, which still run one at a time.

---

## Final UI/UX pass

before releasing the app to the public, we need to do a final pass through the UI/UX to ensure it is polished and ready for release. reimagine the UI with a strong UI/UX skill. and make sure app is responsive and usable on mobile divices.

---

## Unplanned (far out)

Outside the numbered track. Revisit after planned milestones and backlog pressure ease.

0. Improve testing coverage and add more tests for the app.

1. Native mobile apps (iOS / Android), separate from responsive web + PWA

2. Archive & downloads (v3)

Heavy infra; scheduled after interactive product surfaces.

**Deliverables**

- Opt-in local cache of bookmarked pages
- Download / export: PDF, Markdown (reader mode), images, audio, video
- Separate storage volume + background job queue
- Deploy: archive worker + volume mount convention

3. Improved gamification (points / badges on top of likes & ratings)

4. Browser extension

Lowest numbered priority among the planned interactive surfaces.

**Deliverables**

- Save current tab to Linkdean
- Quick file-to-category from the extension

5. can we get around cloudflare bot protection?

6. Easter eggs with Konami-style key sequences ([@arsams/konami-codes-react](https://www.npmjs.com/package/@arsams/konami-codes-react), needs React 19 support first)