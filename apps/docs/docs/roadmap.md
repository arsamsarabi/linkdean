---
id: roadmap
title: Roadmap
sidebar_position: 5
---

# Roadmap

What is planned for Linkdean, in rough order. Everything already shipped is on the [features](/docs/features) page, and each release is in the [changelog](/docs/changelog).

Plans move as the beta teaches us what matters. Want something sooner, or something that is not here? Say so in [GitHub Discussions](https://github.com/arsamsarabi/linkdean/discussions).

## Now: running the public beta well

- **Browser extension in both stores.** Live on the [Chrome Web Store](https://chromewebstore.google.com/detail/ielbdodidimpcolkjhkjdaifpfibolhf); the Firefox add-on is in review. Edge installs from the Chrome listing.
- **Dashboard upgrades.** Choose how each pinned item looks (a small, medium or large card, or just its icon), and sections that fill themselves with your latest bookmarks or the ones you open most.

## Next: once the beta settles

Archiving and AI assistance come first. They are the biggest gaps on the [comparison page](/docs/compare).

- **Archive and downloads.** Keep a copy of every page you bookmark, so a link that dies is not lost.
  - A single-file HTML copy and a PDF of each page, taken when you save it. Opt-in, and counted against your storage.
  - Search inside the pages themselves, not only titles, notes and tags.
  - Download any copy as HTML, PDF or reader-mode Markdown, or include all of them in your export.
  - When a link breaks, open its saved copy instead.
  - Copies of vault bookmarks are encrypted with the vault and kept out of search.
- **AI assistance, on your terms.** Off by default, and you choose who runs it: your own API key, or a local model on your own server. The hosted service will include a small free allowance.
  - Tag and folder suggestions as you save and file.
  - A short summary of each bookmark.
  - Search by meaning ("that article about…"), not only by the words in it.
  - Hints for related and near-duplicate bookmarks.
  - Suggestions wait for your click. Vault contents are never sent to a model.
- **Your bookmarks as memory for AI agents.** Turn a folder or tag into an [Agent Skill](https://agentskills.io) your coding or research agent can use: download it, or keep it fresh with an API key. Vaults are never included.
- **Email digests and one-click unsubscribe.** A daily digest for the chattier notifications (edits, likes, comments), and an unsubscribe link for each kind in every email.
- **Account and legal polish.** Tighter checks around deleting single sign-on accounts, a prompt to accept updated Terms, and a legal review of the Terms and Privacy Policy before anything costs money.
- **More ways to sign in.** Passkeys, and ready-made sign-in with Google, Microsoft, GitHub and Authentik. Any standard OpenID Connect provider already works on a self-hosted instance.
- **An API to build on.** API keys already work. Next: a published OpenAPI reference on this site, one error format across every endpoint, caching headers, and optional tracing.
- **Plans and pricing for the managed service.** Deciding what stays free, what a paid plan would add, and the limits of each. Self-hosting stays free for non-commercial use.
- **Privacy-respecting usage statistics.** Aggregate counts only (active users, bookmarks saved, features used), nothing per person and no page content or addresses. Off by default when you self-host.
- **A slimmer sidebar.** On tablets and wider, a collapsed sidebar becomes a narrow strip of icons instead of disappearing.

## Later

- **Organizations and teams.** Shared team spaces with roles, restricted folders, invites and an audit log, while personal libraries stay private. Then invite links, verified domains, SAML and SCIM single sign-on, and team vaults.
- **Other languages.** The interface translated, starting with making every string translatable.

## Far out

Ideas we want to get to, without a date.

- **End-to-end encrypted vaults**, where the server never sees the key.
- **Native mobile apps** for iOS and Android, beyond the installable web app.
- **A desktop app** for macOS, Windows and Linux.
- **Previews for sites behind bot protection**, which block the screenshot today.
- **Moving off the home server** to a VPS, then to multi-server cloud hosting, as the service grows.
- **A welcome email**, **badges for likes and ratings**, a **FAQ** on this site, and a few **easter eggs**.
