---
id: compare
title: Linkdean compared
sidebar_position: 4.5
description: How Linkdean differs from Raindrop.io, Linkwarden, Karakeep and linkding, and when one of them is the better pick.
---

# Linkdean compared

Good bookmark managers already exist. This page is an honest look at where Linkdean fits beside four of them, including where another tool is the better choice.

:::note
Checked in November 2026 against each project's own site and README. These tools move fast, so confirm anything that decides it for you on their pages. Spotted something out of date? [Tell us](https://github.com/arsamsarabi/linkdean/discussions).
:::

## At a glance

|  | **Linkdean** | **Raindrop.io** | **Linkwarden** | **Karakeep** | **linkding** |
| --- | --- | --- | --- | --- | --- |
| Self-host | Yes, Docker | No | Yes, Docker | Yes, Docker, Kubernetes | Yes, Docker |
| Hosted | Free public beta | Free and Pro plans | Paid cloud plan | Self-host first | Self-host first |
| License | PolyForm Noncommercial (source-available) | Proprietary | AGPL-3.0 | AGPL-3.0 | MIT |
| Organize with | Nested folders and tags; one bookmark in several folders | Nested collections and tags | Collections, sub-collections and tags | Lists and tags | Tags |
| Full-text search | Titles, URLs, descriptions, notes, tags | Page contents (Pro) | Page contents | Page contents, plus semantic search | Titles, descriptions, notes, tags |
| Page archiving | No: previews only | Permanent copies (Pro) | Screenshot, PDF, single HTML, reader view | Full page, plus video | HTML snapshot or Internet Archive |
| AI tagging | No | Yes (Pro) | Optional, local | Yes, OpenAI or local models | No |
| Sharing | Folders and dashboards with other accounts (read or write), unlisted public lists | Collections, collaborators | Collections with per-member permissions, public collections | Collaborative lists | With other users or guests |
| Encrypted folders | Yes: vaults with their own passphrase | No | No | No | No |
| Broken links and duplicates | Broken-link checks and a duplicate finder | Both (Pro) | Optional duplicate prevention | List of links that failed to crawl | Not listed |
| Phone | Installable web app, share sheet, iOS Shortcut | Native apps | Native apps | Native apps, offline reading | Installable web app |
| Browser extension | Chrome and Edge; Firefox in review | All major browsers | Yes | Chrome, Firefox, Safari | Chrome, Firefox |
| Single sign-on | Optional OIDC | No | Self-hosted and enterprise | Yes | OIDC or auth proxy |

## Where Linkdean is different

- **Vaults.** Put private folders behind their own passphrase. Contents are encrypted at rest (Argon2id and AES-GCM), never sent to the search index, and stay locked on other devices until you unlock them there. None of the others offer this.
- **Filing Unsorted one by one.** Save now, file later: a card at a time, with folders and tags you already used on the same site offered as one-click picks. Keyboard on desktop, swipe on a phone.
- **A bookmark can live in several folders.** No copying a link into two places and keeping both up to date.
- **Dashboards.** Pin the links you open every day into sectioned start pages, or let sections fill themselves with your latest, most opened or recently visited bookmarks. Share them read-only or editable.
- **Sharing that respects people.** Invite another account or an email address to a folder or dashboard, and they choose to accept. Likes, comments and ratings live on shared items only; follow, mute and block work as you would expect, and everything starts private.
- **Library health without a paid tier.** Broken-link checks, a duplicate finder and a duplicate review on import are part of every account.

## Where another tool fits better

- **You want a copy of every page you save.** Linkdean stores page previews, not the page itself. If link rot worries you, [Linkwarden](https://linkwarden.app) and [Karakeep](https://karakeep.app) archive full pages, and [Raindrop.io](https://raindrop.io) does on Pro. Page copies, downloads and search inside pages are [next on the roadmap](/docs/roadmap#next-once-the-beta-settles).
- **You want AI to tag and summarize for you.** Karakeep is built around it, Linkwarden offers it with local models, and Raindrop.io has it on Pro. Linkdean has no AI features today; opt-in AI assistance, with your own key or a local model, is [next on the roadmap](/docs/roadmap#next-once-the-beta-settles).
- **You want highlights, annotations or RSS.** Linkwarden, Karakeep and Raindrop.io (Pro) have them; Linkdean does not.
- **You want native phone apps.** Raindrop.io, Linkwarden and Karakeep ship them. Linkdean is an installable web app with share-sheet saving.
- **You want an OSI open-source license.** Linkwarden and Karakeep are AGPL-3.0 and linkding is MIT. Linkdean's source is public under the PolyForm Noncommercial license: free for personal and non-commercial use, and forks are welcome, but commercial use needs a separate license.
- **You want the smallest possible server.** linkding is a single lightweight container. Linkdean needs Postgres alongside it, and Meilisearch for search.
- **You want a finished product today.** Linkdean is in public beta. Export your library now and then, and expect rough edges.

## Moving to Linkdean

Import from a browser export (Netscape HTML), a **Pocket** CSV, a **Raindrop.io** CSV or a Linkdean JSON file. Most tools, including Linkwarden, Karakeep and linkding, export browser HTML, so that file works for them too. Linkdean shows what it will import and flags duplicates before anything is saved; see [Getting started](/docs/getting-started).

Leaving is just as easy: export everything as Linkdean JSON or browser HTML from **Config options**.

Page archiving and AI assistance are next on the [roadmap](/docs/roadmap); other planned work that closes gaps above is there too.
