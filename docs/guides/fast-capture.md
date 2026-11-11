# Saving pages fast

Four ways to save the page you are on without opening Linkdean and pasting a link. All of them end on the quick save form (`/save`): the address and title are filled in, the page's description and keyword tags are fetched when no title came along, and the folder you used last is already picked. **Save to Unsorted** saves in one tap so you can file it later. A page that is already in your library opens that bookmark instead of saving a second copy.

## Browser extension (Chrome, Edge, Firefox)

Store listings are in review; until they are live, build from `apps/extension` and load it unpacked (its README says how). Capability summary: [features.md](../product/features.md).

1. Click the Linkdean button in the toolbar and choose **Sign in**. Keep `app.linkdean.app`, or type the address of your own Linkdean.
2. Allow the extension to reach that site when the browser asks. It asks for that one site only.
3. A Linkdean window opens. Sign in if you need to, then choose **Allow**.

Then, on any page:

- **The toolbar button** (or **Alt+Shift+S**) shows the page's title, the text you selected as the description, your folders and tags. **Save** files it; **Save to Unsorted** saves it with no folder. A page you already saved shows **Open in Linkdean** instead.
- **Right-click → Save page to Linkdean**, or **Save link to Linkdean** on a link, saves to Unsorted at once. A ✓ on the toolbar button confirms it.

The extension holds its own API key, listed as "Browser extension" under **Settings → API keys**. **Sign out** in the extension revokes it.

## Bookmarklet (desktop browsers)

In Linkdean, open **Settings → Fast capture**.

- **Chrome, Edge, Firefox:** show the bookmarks bar, then drag **Save to Linkdean** onto it.
- **Safari:** click **Copy code**, bookmark any page, edit that bookmark and paste the code as its address.

Click it on any page. Linkdean opens in a small window with the page's address, title and the text you had selected (as the description). The window closes after you save. If a site blocks popups, Linkdean opens in the same tab instead.

The bookmarklet uses the session you are signed in with in that browser. Signed out, you sign in in the small window and the save carries on.

## Share → Linkdean (Android, desktop Chrome and Edge)

Install Linkdean as an app: in Chrome on Android, **⋮ → Add to Home screen** (or **Install app**); on desktop, the install icon in the address bar. Linkdean then shows up in the system share sheet. Share a link from any app and the quick save form opens with it.

Apps share in different ways. Some send the link as text; Linkdean takes the first web address in the text as the link and the rest as the description.

## iPhone and iPad: a Shortcut

iOS does not let web apps receive shares, so use the Shortcuts app. Pick one of these.

### Open quick save (no API key)

Uses the session you are signed in with in Safari, and shows the form so you can pick a folder and tags.

1. In **Shortcuts**, tap **+** to make a new shortcut and name it **Save to Linkdean**.
2. Tap the **ⓘ** (details) button, turn on **Show in Share Sheet**, and set the input to **URLs** and **Safari web pages**.
3. Add the action **URL Encode** with **Shortcut Input**.
4. Add the action **Text** with: `https://app.linkdean.app/save?url=` followed by the **URL Encoded Text** variable. On a self-hosted instance, use your own address.
5. Add the action **Open URLs** with that **Text**.

Share any page → **Save to Linkdean** opens the form in Safari.

### Save straight to Unsorted (API key)

Saves without opening anything, for filing later. Linkdean fetches the title, preview and site icon in the background.

1. In Linkdean, create a key under **Settings → API keys** and copy it. Treat it like a password: anyone with it can read and change your bookmarks.
2. In **Shortcuts**, make a new shortcut, name it **Save to Linkdean (Unsorted)**, and turn on **Show in Share Sheet** for **URLs** and **Safari web pages**, as above.
3. Add the action **Get Contents of URL**:
   - URL: `https://app.linkdean.app/api/bookmarks` (or your instance's address followed by `/api/bookmarks`)
   - Method: **POST**
   - Headers: `Authorization` = `Bearer ` followed by your key
   - Request Body: **JSON**, with the field `url` (Text) set to **Shortcut Input**
4. Optionally add **Show Notification** with "Saved to Linkdean".

If the shortcut fails with a 401, the key has been revoked or mistyped; with a 403, the account is banned or over a limit set by the instance's admin.
