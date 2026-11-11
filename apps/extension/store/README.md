# Store submission kit

Everything the Chrome Web Store and addons.mozilla.org (AMO) ask for on the first submission. Copy each field as it stands. Images are in [`images/`](images); re-render them with `bash store/render/render.sh` after a popup change.

## Before submitting

1. **Release the server first.** Reviewers sign in to `https://app.linkdean.app`, and the store listings link to its privacy policy. The live instance needs this release's `/connect/extension` page, the `/api/auth/extension/*` endpoints and the privacy policy's "Browser extension" section. Until then the extension says "app.linkdean.app does not support the extension yet".
2. **Create a reviewer account** on app.linkdean.app with a few folders and tags, and API keys allowed. Put its email and password in the test notes below. Don't use your own account.
3. **Build the uploads** from `apps/extension` on the merged `main`:

   ```bash
   pnpm install --frozen-lockfile
   pnpm zip            # .output/linkdean-extension-<version>-chrome.zip
   pnpm zip:firefox    # .output/linkdean-extension-<version>-firefox.zip and -sources.zip
   pnpm lint:firefox   # Mozilla's checks: 0 errors expected
   ```

   The version comes from `package.json` (`0.1.0` for the first submission). Each later upload needs a higher version.

## Shared listing text

**Name:** Linkdean

**Short description** (Chrome summary, 132 characters max; AMO summary, 250 max):

> Save the page you're on to your Linkdean library, with its title, selected text, a folder and tags. Works with self-hosted too.

**Description:**

> Linkdean is a bookmark manager you can use at linkdean.app or run on your own server. This extension saves pages to it without leaving the page you're reading.
>
> Click the Linkdean button (or press Alt+Shift+S) and the page's title is filled in, with any text you selected as the description. Pick a folder (your last one is already chosen) and add tags, then Save. In a hurry? Save to Unsorted and file it later.
>
> Right-click a page or a link and choose "Save page to Linkdean (Unsorted)" or "Save link to Linkdean (Unsorted)" to save it without opening anything. A check on the toolbar button tells you it's in.
>
> Open a page you've already saved and the extension says so, with a button to open it in Linkdean, instead of saving a second copy.
>
> Signing in happens on the Linkdean website: sign in as usual and choose Allow. The extension gets its own key, which you can see and revoke under Settings → API keys. Signing out of the extension revokes it.
>
> Self-hosting? Type your Linkdean's address when you sign in. The extension asks for access to that one address only, and talks to nothing else.
>
> Privacy: the extension sends the address, title and selected text of the pages you choose to save to the Linkdean you signed in to, and nothing to anyone else. It doesn't read or send the pages you visit otherwise.

**Privacy policy URL:** `https://app.linkdean.app/privacy`

**Homepage / website:** `https://linkdean.app`

**Support:** `https://github.com/arsamsarabi/linkdean/issues`, or the support email of the store account.

**Images:**

| File                                | Use                                                                    |
| ----------------------------------- | ---------------------------------------------------------------------- |
| `images/screenshot-save.png`        | Screenshot 1 (1280×800)                                                |
| `images/screenshot-menu.png`        | Screenshot 2                                                           |
| `images/screenshot-saved.png`       | Screenshot 3                                                           |
| `images/screenshot-signin.png`      | Screenshot 4                                                           |
| `images/promo-small-440x280.png`    | Chrome small promo tile                                                |
| `images/promo-marquee-1400x560.png` | Chrome marquee promo tile                                              |
| `public/icon/128.png`               | Store icon (Chrome takes it from the zip; AMO uses the manifest icons) |

## Chrome Web Store

Upload `linkdean-extension-<version>-chrome.zip` at the [developer dashboard](https://chrome.google.com/webstore/devconsole), then fill in each tab.

**Store listing**

- Category: **Productivity → Tools**.
- Language: English.
- Description, icon, screenshots, small promo tile, marquee promo tile: as above.
- Official URL: leave unset unless linkdean.app is verified in Search Console under the publisher account.
- Homepage URL: `https://linkdean.app`. Support URL: the GitHub issues link.

**Privacy**

- **Single purpose:**

  > Save the page in the current tab, or a link on it, as a bookmark in the user's own Linkdean library.

- **Permission justifications:**

  | Permission                 | Justification                                                                                                                                                                                                                                                                                          |
  | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
  | `activeTab`                | Reads the address and title of the current tab, only when the user clicks the toolbar button, uses the right-click menu or presses the shortcut, so that page can be saved.                                                                                                                            |
  | `contextMenus`             | Adds "Save page to Linkdean (Unsorted)" and "Save link to Linkdean (Unsorted)" to the right-click menu.                                                                                                                                                                                                |
  | `scripting`                | Reads the text the user selected on the current tab, after they click the toolbar button, to use as the bookmark's description. During sign-in it also runs a small script on the user's own Linkdean approval page, in the sign-in window the extension opened, to receive the one-time sign-in code. |
  | `storage`                  | Keeps the sign-in (the Linkdean address, the extension's API key and the account email) and the last folder used, on the user's device.                                                                                                                                                                |
  | Host permission (optional) | Requested at sign-in for the single Linkdean address the user chooses (linkdean.app or their own server), to call its API. No other site is requested.                                                                                                                                                 |

- **Remote code:** No, I am not using remote code. All code is in the package.
- **Data usage**, check:
  - Personally identifiable information: the account email, shown in the popup.
  - Authentication information: the API key the extension uses.
  - Web history: the address and title of pages the user saves, and of the current tab when the popup checks whether it is already saved.
  - Website content: the text the user selects, saved as the description.

  Then tick all three certifications (not sold, not used for unrelated purposes, not used for creditworthiness).

- Privacy policy URL: `https://app.linkdean.app/privacy`.

**Distribution**

- Visibility: Public. Regions: all. Free.

**Test instructions** (Account → Test instructions, or the submission notes):

> Click the toolbar button, keep https://app.linkdean.app, click Sign in and allow access to app.linkdean.app. In the window that opens, sign in with the test account below and choose Allow; the window closes. Open any web page, select some text, click the toolbar button and Save. Right-click a page for "Save page to Linkdean (Unsorted)".
>
> Test account: <email> / <password>

Publisher: `dev.linkdean@gmail.com`, non-trader. After the item is approved, note its ID (the 32-letter string in its store URL) for automated publishing.

## Firefox (AMO)

Submit at [addons.mozilla.org/developers](https://addons.mozilla.org/developers/) with arsam@linkdean.app → Submit a New Add-on → **On this site** (listed).

1. Upload `linkdean-extension-<version>-firefox.zip`. Compatible platforms: Firefox only. Android is not supported; it has no window for the sign-in. The manifest needs Firefox 140 or later, the first version that shows the built-in data collection consent; older versions would need a consent screen of our own.
2. **Source code:** Yes, upload `linkdean-extension-<version>-sources.zip`. The build is bundled and minified, so Mozilla requires it.
3. Listing:
   - Name, summary, description: as above.
   - Categories: **Bookmarks**. Optional second: Other.
   - Support email or site, homepage: as above.
   - License: **All Rights Reserved**. PolyForm Noncommercial is not in AMO's list, and it is not an open source license either.
   - Privacy policy: Yes, link `https://app.linkdean.app/privacy` or paste its "Browser extension" section.
   - Screenshots: the four images above. AMO has no promo tile.
4. **Data collection:** the manifest declares `browsingActivity` and `websiteContent` as required. If the form asks again, confirm those two and nothing else.
5. **Notes to reviewer:**

   > Built with WXT (Vite). To reproduce the upload from the source zip, with Node 22 and pnpm 10:
   >
   > ```
   > pnpm install --frozen-lockfile
   > pnpm zip:firefox
   > ```
   >
   > The result is .output/linkdean-extension-<version>-firefox.zip.
   >
   > web-ext lint warnings come from bundled libraries: the `Function` check is Zod's test for whether eval is allowed (it is caught and not used under the extension CSP), and the `innerHTML` assignments are React DOM's. The extension's own code uses neither.
   >
   > To test: click the toolbar button, keep https://app.linkdean.app, Sign in, allow access to that site, sign in in the window with the account below and choose Allow. Then save any page from the button or the right-click menu.
   >
   > Test account: <email> / <password>

## Publishing from release tags

After the first manual submissions, the Release workflow submits each new version (see [the extension README](../README.md#release)). It runs `wxt submit` in the `extension-stores` GitHub environment and needs the secrets below. The Chrome Web Store API can't create an item, so the first upload to each store stays manual.

**Chrome Web Store** (API v2, with a service account, so there is no OAuth refresh token to expire):

1. In the [Google Cloud console](https://console.cloud.google.com/), signed in as the publisher account (`dev.linkdean@gmail.com`), create a project (for example `linkdean-extension`) and enable the **Chrome Web Store API** in it.
2. **IAM & Admin → Service accounts → Create service account**, for example `cws-publisher`. Grant it no roles.
3. Open the service account, **Keys → Add key → Create new key → JSON**. Keep the downloaded file private; delete it once the secrets are set.
4. In the [Developer Dashboard](https://chrome.google.com/webstore/devconsole), **Account**: add the service account's email (`client_email` in the JSON). A publisher can have only one. The publisher account needs 2-step verification.
5. Note the **publisher ID** (Dashboard → Publisher → Settings) and the **item ID** (32 letters, on the item's Package tab).

**AMO:** at [addons.mozilla.org/developers/addon/api/key/](https://addons.mozilla.org/developers/addon/api/key/), signed in as the add-on's owner, generate credentials: a JWT issuer and a JWT secret. The add-on ID, `firefox@linkdean.app`, is in the workflow.

**GitHub:** create the environment and set its secrets with `gh`, from the folder holding the key file:

```bash
repo=arsamsarabi/linkdean
env=extension-stores
gh api -X PUT "repos/$repo/environments/$env" >/dev/null
gh secret set CHROME_PUBLISHER_ID --repo "$repo" --env "$env"                  # paste the publisher ID
gh secret set CHROME_EXTENSION_ID --repo "$repo" --env "$env"                  # paste the item ID
jq -r .client_email key.json | gh secret set CHROME_SERVICE_ACCOUNT_CLIENT_EMAIL --repo "$repo" --env "$env"
jq -r .private_key key.json | gh secret set CHROME_SERVICE_ACCOUNT_PRIVATE_KEY --repo "$repo" --env "$env"
gh secret set FIREFOX_JWT_ISSUER --repo "$repo" --env "$env"                   # paste the JWT issuer
gh secret set FIREFOX_JWT_SECRET --repo "$repo" --env "$env"                   # paste the JWT secret
```

`jq -r` writes the private key with real line breaks, which the signer needs. To approve each submission by hand, add yourself as a required reviewer under **Settings → Environments → extension-stores**.

**Check the credentials without submitting**, from `apps/extension` after `pnpm zip && pnpm zip:firefox`, with the same values exported as environment variables plus `CHROME_API_VERSION=v2` and `FIREFOX_EXTENSION_ID=firefox@linkdean.app`:

```bash
v=$(jq -r .version package.json)
pnpm exec wxt submit --dry-run \
  --chrome-zip .output/linkdean-extension-$v-chrome.zip \
  --firefox-zip .output/linkdean-extension-$v-firefox.zip \
  --firefox-sources-zip .output/linkdean-extension-$v-sources.zip
```
