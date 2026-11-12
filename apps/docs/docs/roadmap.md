---
id: roadmap
title: Roadmap
sidebar_position: 5
---

# Roadmap

What is planned for Linkdean, in rough order. Everything already shipped is on the [features](/docs/features) page, and each release is in the [changelog](/docs/changelog).

Plans move as the beta teaches us what matters. Want something sooner, or something that is not here? Say so in [GitHub Discussions](https://github.com/arsamsarabi/linkdean/discussions).

## Now: running the public beta well

- **Browser extension in both stores.** Live on the Chrome Web Store; the Firefox add-on is in review. Edge uses the Chrome build.
- **Dashboard upgrades.** Choose how each pinned item looks (a small, medium or large card, or just its icon), and sections that fill themselves with your latest bookmarks or the ones you open most.

## Next: once the beta settles

- **Email digests and one-click unsubscribe.** A daily digest for the chattier notifications (edits, likes, comments), and an unsubscribe link for each kind in every email.
- **Account and legal polish.** Tighter checks around deleting single sign-on accounts, a prompt to accept updated Terms, and a legal review of the Terms and Privacy Policy before anything costs money.
- **More ways to sign in.** Passkeys, and ready-made sign-in with Google, Microsoft, GitHub and Authentik. Any standard OpenID Connect provider already works on a self-hosted instance.
- **An API to build on.** API keys already work. Next: a published OpenAPI reference on this site, one error format across every endpoint, caching headers, and optional tracing.
- **Plans and pricing for the managed service.** Deciding what stays free, what a paid plan would add, and the limits of each. Self-hosting stays free for non-commercial use.
- **Privacy-respecting usage statistics.** Aggregate counts only (active users, bookmarks saved, features used), nothing per person and no page content or addresses. Off by default when you self-host.

## Later

- **Archive and downloads.** Keep a copy of the pages you bookmark, so a link that dies is not lost, and download them as PDF, reader-mode Markdown or media.
- **Organizations and teams.** Shared team spaces with roles, restricted folders, invites and an audit log, while personal libraries stay private. Then invite links, verified domains, SAML and SCIM single sign-on, and team vaults.
- **Other languages.** The interface translated, starting with making every string translatable.

## Far out

Ideas we want to get to, without a date.

- **AI features**, such as auto-tagging, summaries and semantic search, with a limited free tier and bring-your-own-key.
- **End-to-end encrypted vaults**, where the server never sees the key.
- **Native mobile apps** for iOS and Android, beyond the installable web app.
- **A desktop app** for macOS, Windows and Linux.
- **Previews for sites behind bot protection**, which block the screenshot today.
- **Moving off the home server** to a VPS, then to multi-server cloud hosting, as the service grows.
- **A welcome email**, **badges for likes and ratings**, a **FAQ** on this site, and a few **easter eggs**.
