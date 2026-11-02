# Browser QA: Tier 3 defect fixes

Manual checks for the Tier 3 fixes in [POSSIBLE_DEFECTS.md](./POSSIBLE_DEFECTS.md). Automated tests cover the logic behind each one. These checks confirm what a person sees in the browser.

Tick each box as you go. If a step fails, note the item ID and what you saw.

## Setup

1. Start the dev stack: `docker compose -f deploy/docker/docker-compose.dev.yml up --build`.
2. Open http://localhost:5173 and hard reload (Cmd+Shift+R) so the latest web build loads.
3. Use two accounts:
   - **A** (`qa-a@example.test`): signs in for most checks.
   - **B** (`qa-b@example.test`): only for the sharing checks.
4. As A, create:
   - folders `Work` and `Music`;
   - five bookmarks, two of them in `Work` and some with tags;
   - one vault named `Private`, with passphrase `vault-passphrase` and a recovery key;
   - a global passphrase `global-passphrase` (Vaults page → Global passphrase → Turn on), with `Private` switched to "Opens with the global passphrase".

---

## 1. Library and dashboard

### M43: Grid view follows the named folder

- [ ] Library → switch the view to **Grid**.
- [ ] Press Cmd+K and choose the `Work` folder.
- [ ] **Expect:** only `Work`'s bookmarks show, above them a line reads "In **Work**", and **Add bookmark** is enabled.
- [ ] Click **Show all**. **Expect:** the whole library shows again and the "In" line is gone.
- [ ] Jump to `Work` again, search for something that matches nothing, and click **Clear search & filters**. **Expect:** this also clears the folder.

### M39: Selection drops items that leave the list

- [ ] Library → **Select**, then tick three bookmarks.
- [ ] Type a search that hides two of them.
- [ ] **Expect:** the selected count drops to 1.
- [ ] Click **Delete** in the bulk bar. **Expect:** the confirmation names only the visible bookmark.
- [ ] Cancel.

### M36: Bulk add keeps existing tags and folders

- [ ] Give bookmark X the tag `old` and the folder `Work`.
- [ ] Library → **Select** → tick X and one other bookmark → **Tag** → choose **Add**, enter `new`, and apply.
- [ ] **Expect:** X has both `old` and `new`.
- [ ] With the same selection, choose **Categories** → add `Music` → apply.
- [ ] **Expect:** X is in both `Work` and `Music`.
- [ ] In a second tab, add tag `extra` to X. Back in the first tab, *without reloading*, bulk-add tag `more` to X.
- [ ] **Expect:** X keeps `extra`. Before the fix, the stale list in the first tab removed it.

### M50: Destructive bulk tag replace asks first

- [ ] **Select** → tick two bookmarks → **Tag** → choose **Replace**, leave the field empty.
- [ ] **Expect:** Apply is blocked.
- [ ] Enter `x` and apply. **Expect:** a "Replace tags?" confirmation appears.
- [ ] Press Esc once. **Expect:** only the confirmation closes and the drawer stays open (see also M45).

### M37: A blank section title is refused

- [ ] Dashboard → **Edit layout** → **Add section**, pin a bookmark to it, and clear its title.
- [ ] Click Save.
- [ ] **Expect:** a red "Every section needs a title." message, and the editor stays open.
- [ ] Give the section a title and save. **Expect:** "Dashboard saved", with the pin still in that section.

### M45: Esc closes only the dialog on top

Check each pair below. Each time, press Esc **once**.

- [ ] Bulk edit drawer → Replace tags confirmation. **Expect:** only the confirmation closes.
- [ ] Folder actions → Share → **Public link** → rotate link → confirmation. **Expect:** only the confirmation closes.
- [ ] Bookmark → comments (social drawer) → delete your comment → confirmation. **Expect:** only the confirmation closes.
- [ ] Edit a bookmark → upload a custom preview image → crop dialog. **Expect:** only the crop dialog closes, and your unsaved edits in the bookmark drawer remain.
- [ ] Profile → edit → upload an avatar → crop dialog. **Expect:** only the crop dialog closes.
- [ ] Settings → Users (as admin) → moderation drawer → an action that asks to confirm. **Expect:** only the confirmation closes.
- [ ] After each one, press Esc a second time. **Expect:** now the drawer closes.

### M14: Duplicating a folder skips Trash

- [ ] In `Work`, create a subfolder `Old`, then move `Old` and one `Work` bookmark to Trash.
- [ ] Folder actions for `Work` → **Duplicate**.
- [ ] **Expect:** `Work (copy)` has no `Old` subfolder and no trashed bookmark, only the live ones.

---

## 2. Import

Save this as `qa-import.html`:

```html
<DL><p>
<DT><H3>AC/DC</H3>
<DL><p>
<DT><H3>Live</H3>
<DL><p>
<DT><A HREF="https://example.com/a">A</A>
</DL><p>
</DL><p>
<DT><H3>Rock</H3>
<DL><p>
<DT><H3>Live</H3>
<DL><p>
<DT><A HREF="https://example.com/b">B</A>
</DL><p>
</DL><p>
</DL><p>
```

### M26 / M27: Folders import as a tree, names intact

- [ ] Settings → **Import / Export** → Format **Netscape HTML** → upload `qa-import.html` → Import.
- [ ] **Expect:** a folder named exactly `AC/DC` (not `DC`), with `Live` inside it.
- [ ] **Expect:** a separate `Rock` folder with its own `Live` inside it. The two `Live` folders stay apart, with A in one and B in the other.
- [ ] Export as Linkdean JSON. In the file, move a child folder's entry above its parent's, then import it into account B.
- [ ] **Expect:** the folders nest the same as in A.

### M28: A failed import says so

- [ ] Import a Linkdean JSON file whose dashboard has a section titled `" "`. You can edit an export by hand to make one.
- [ ] **Expect:** an error message, not success.
- [ ] **Expect:** the bookmarks that did import show up in search a moment later.

---

## 3. Vaults

### M6: Sensitive vault actions ask for the passphrase

- [ ] Vaults → unlock `Private` → **Replace recovery key**.
- [ ] **Expect:** a dialog with the warning text and a passphrase field, and **Replace key** is disabled until you type a passphrase.
- [ ] Type a wrong passphrase. **Expect:** an error, and the dialog stays open.
- [ ] Type `vault-passphrase`. **Expect:** the new recovery key is shown once.
- [ ] Global passphrase card: with **Current passphrase** empty, **Turn off** is disabled.
- [ ] Enter a wrong current passphrase and click **Turn off**. **Expect:** an error, and the card still shows it as on.
- [ ] Enter `global-passphrase` and click **Turn off**. **Expect:** "Global passphrase turned off".
- [ ] Move a bookmark into `Private`, delete it, then **Lock** `Private`.
- [ ] Trash → **Delete forever** on that bookmark. **Expect:** a vault-locked error.
- [ ] Unlock `Private` and try again. **Expect:** it is deleted.

### M9: Moving a profile-visible folder into a vault

- [ ] Set `Music` to "Who can see this" → **Followers**.
- [ ] Folder actions → **Move to vault** → `Private`.
- [ ] **Expect:** the move succeeds. Before the fix, it failed with a server error.
- [ ] **Expect:** `Music` no longer appears on your profile.

### M44: "Unlock vault for search" stays dismissed

- [ ] With a vault locked, search the Library until the "Unlock vault for search" prompt appears, then dismiss it.
- [ ] Search again in the same tab. **Expect:** the prompt does not come back.
- [ ] Open a new tab. **Expect:** it may ask once in that tab.

### M23: Huge images are refused

- [ ] Upload an image over 40 megapixels (for example 8000×6000) as an avatar or custom preview.
- [ ] **Expect:** a clear "image too large" error. The app stays responsive.
- [ ] A normal phone photo (12 MP) still uploads.

### L12: A removed preview stays removed

Needs `BROWSER_WS_URL` (screenshots) enabled in the dev stack. Skip this item if screenshots are off.

- [ ] On a bookmark that has a screenshot preview, edit it → preview → **Remove**.
- [ ] Restart the API container (`docker compose -f deploy/docker/docker-compose.dev.yml restart api`).
- [ ] **Expect:** after a minute, the bookmark still has no screenshot.
- [ ] Change the bookmark's URL. **Expect:** a new screenshot is taken.

---

## 4. Accounts and sign-in

### M1: Password change signs out other sessions

- [ ] Sign in as A in two browsers, or in a normal and a private window.
- [ ] In window 1: Profile → **Change password**, with **Also revoke all API keys** ticked.
- [ ] **Expect:** window 1 stays signed in. Window 2 lands on `/login` at its next action. API keys are gone from the API keys panel.

### Session sync across tabs

- [ ] Open the app in two tabs and sign out in one.
- [ ] **Expect:** the other tab goes to `/login` without a reload.

### M49: Invite gate on Register

- [ ] With registration set to invite-only, open `/register`.
- [ ] **Expect:** the **Invite code** gate shows.
- [ ] Type a code character by character. **Expect:** the field keeps focus and the gate does not flicker away.
- [ ] Open `/register?invite=<code>` directly. **Expect:** it goes straight to the form.

### L24: Deep links survive a reload

- [ ] Open `/library`, `/vaults` and a dashboard URL, and press reload on each.
- [ ] **Expect:** the page loads, not a 404. On the production image (port 8080), `/api/nope` returns a JSON 404.

### H11–H13, M35, L20: OIDC sign-in

Needs an OIDC provider, such as Authentik, with `OIDC_ISSUER`, `OIDC_CLIENT_ID` and `OIDC_CLIENT_SECRET` set. Register the redirect URI `{APP_URL}/api/auth/oidc/callback` with the provider, using the dev stack's `APP_URL`.

- [ ] Login → **Sign in with OIDC** → sign in at the provider.
- [ ] **Expect:** you land on `/` signed in.
- [ ] Start the flow, then open the callback URL from another browser.
- [ ] **Expect:** `/login` with an error message, not a signed-in page.
- [ ] With registration closed, sign in with a provider account that has no Linkdean account.
- [ ] **Expect:** `/login` with an error, and no account is created.
- [ ] Sign out. **Expect:** back on `/login`, and reloading does not sign you back in.

---

## Reporting

For each failure, note the item ID, the step, and what you saw, with a screenshot if it helps. Failures go back into [POSSIBLE_DEFECTS.md](./POSSIBLE_DEFECTS.md) as reopened items.
