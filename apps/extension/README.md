# Linkdean browser extension

Save the current tab to Linkdean from Chrome (and Edge) or Firefox. One codebase, built with [WXT](https://wxt.dev) as Manifest V3 for both browsers. Works with app.linkdean.app or any self-hosted instance.

What it does, and the sign-in flow: [docs/guides/fast-capture.md](../../docs/guides/fast-capture.md) and the "Browser extension" bullets in [docs/product/features.md](../../docs/product/features.md).

## Develop

Requires Node 22 and pnpm 10.

```bash
pnpm install
pnpm dev            # Chrome, with a fresh profile and live reload
pnpm dev:firefox    # Firefox
```

Sign in to `http://localhost:5173` with the dev stack running (`deploy/docker/docker-compose.dev.yml`).

To load a build in your everyday browser instead:

- **Chrome:** `pnpm build`, then `chrome://extensions` → Developer mode → **Load unpacked** → `.output/chrome-mv3`.
- **Firefox:** `pnpm build:firefox`, then `about:debugging#/runtime/this-firefox` → **Load Temporary Add-on** → `.output/firefox-mv3/manifest.json`.

An unpacked Chrome build gets an extension ID from its folder path, so its address (`chrome-extension://<id>/`) differs from the store build's. The server accepts any Chrome or Firefox extension address, so both work.

## Check

```bash
pnpm format:check && pnpm lint && pnpm typecheck && pnpm test && pnpm build && pnpm build:firefox && pnpm lint:firefox
```

CI runs the same on every push. `lint:firefox` is Mozilla's add-on linter, the checks AMO runs on upload.

## Release

The Release workflow (`.github/workflows/release.yml`) submits the extension to the Chrome Web Store and AMO on a `v*.*.*` tag, but only when its build inputs changed since the previous tag and `version` in `package.json` went up. Tests, `store/` and this README don't count. A build change without a version bump is skipped with a warning, because the stores refuse a version they already have. So to ship the extension, bump `version` in the same PR as the change, then tag a release as usual. Both stores still review each version before it goes live.

Setting up the store credentials: [store/README.md](store/README.md#publishing-from-release-tags).

To build the uploads by hand:

```bash
pnpm zip            # .output/linkdean-extension-<version>-chrome.zip
pnpm zip:firefox    # the Firefox zip and the sources zip AMO asks for
```

Listing text, permission justifications, privacy answers, reviewer notes and images for both stores: [store/README.md](store/README.md).

## Building from source (for addons.mozilla.org review)

1. Install Node 22 and pnpm 10.30.3 (`corepack enable`).
2. In this folder: `pnpm install --frozen-lockfile`.
3. `pnpm build:firefox`. The add-on is in `.output/firefox-mv3`; `pnpm zip:firefox` packs it.

The build bundles React, Mantine and Zod with Vite; nothing is fetched at runtime, and no code is obfuscated.
