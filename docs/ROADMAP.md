# Roadmap

What is still planned. Shipped capabilities live in [FEATURES.md](./FEATURES.md). Scratch ideas: [IDEAS.md](./IDEAS.md).

---

## Backlog 

**Product**

- Pending email invites for bookmark / list / dashboard shares (not public URLs): invite by address; if recipient has no account, they register and claim access
- Broken-link checker (rate-limited worker)
- Full-text notes and optional highlights
- increase the number of icons offered to the user for the categories. paginate the icons so the user can scroll through them. add a search input to the icons page so the user can search for an icon.
- Allow user to upload their own image for the bookmark items. replace the default image with the uploaded image.

**Social**

- Save (“bookmark”) another user’s shared bookmark into own library
- Share a **single bookmark** (not only folder / dashboard)
- Follow / unfollow users
- Block / unblock users
- Mute / unmute users
- Profile + library visibility: public / private / followers-only; hidden profile; private bookmarks/lists
- _(likes, comments, ratings on shared items — already shipped)_

**Sharing**

- Accept / reject share flow: share → notification only; accept adds to account, reject ignores (no immediate library inject)

**Teams**

- Create team; invite / remove members
- View team, members, bookmarks/lists, dashboards
- Team activity, comments, and likes surfaces

**Moderation**

- Admin ban / unban user
- Banned-users list + bulk edit
- Ban reason shown to banned user
- Ban duration: absolute date or N days

**Vault & privacy**

- Client-held vault key option (true E2E; harder search tradeoff)
- Per-category vault passphrases (today: one vault per user)
- Second vault unlock via Konami sequence ([@arsams/konami-codes-react](https://www.npmjs.com/package/@arsams/konami-codes-react))

**Auth & multi-user**

- Passkeys / WebAuthn
- Hardened OIDC (PKCE + discovery) for all providers, not just Authentik-shaped URLs
- Configurable SSO providers: Authentik, Google, Microsoft, GitHub, etc.

**Platform**

- SMTP / outbound email config (for invites + notification delivery) ([react-email](https://www.npmjs.com/package/react-email) for templates)
- Localization / i18n-ready strings (and eventual locale packs)
- Mobile-friendly layout pass
- backend app to write logs for the app. in a structured and standard format. all activity and errors should be logged to file..

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
