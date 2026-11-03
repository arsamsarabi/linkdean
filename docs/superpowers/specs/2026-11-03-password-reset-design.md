# Outbound email, part B: password reset

Approved design, not yet planned. Builds on part A (mail foundation). Vault data is unaffected by a reset: vaults are encrypted with their own passphrases, not the account password, so they simply start locked.

## User flow (mail on)

- The login page links **Forgot password?** to `/forgot-password`: one email field.
- The reply is always the same, for known and unknown addresses alike, and takes about the same time: "If an account exists for that address, we've sent a link. It works for 30 minutes."
- Email `password_reset`: a "Reset your password" button plus the printed link; valid 30 minutes, works once, ignore it if you did not ask.
- The link opens `/reset-password#token=…`. The token is in the fragment, so it never reaches the server's access logs or a `Referer` header.
- The page asks for the new password twice, with an "Also revoke my API keys" checkbox (as on password change).
- A bad, expired or used link shows "This link has expired or was already used" and a button to request a new one.
- On success:
  - every session ends;
  - the browser that reset is signed in and lands on the library;
  - vaults stay locked, passphrases unchanged;
  - email `password_changed`: "Your password was changed at <time>. Not you? Reset it again and tell your admin."
- An account without a local password (SSO only) gets an email saying it signs in with its identity provider, with no link.

## Admin flow (always available; the only route when mail is off)

- Admin → Users → user menu → **Create reset link**. A modal shows the link once, with a Copy button and the note "Send it privately. Anyone with this link can set the password." Valid 24 hours.
- If mail is on, the user also gets a notice email: "An admin created a password reset link for your account."
- Refused for accounts without a local password.
- A banned user can reset; sign-in still refuses them.

## Mail off

The login page shows "Forgot your password? Ask your admin for a reset link." instead of the link.

## Rules

- Tokens are 32 random bytes, stored as SHA-256 hashes, with expiry, used time and whether an admin created them.
- One live token per user: a new link (admin or email) voids the previous one. A normal password change also voids it.
- Single use, enforced in one transaction so two concurrent uses cannot both succeed.
- Expired and used tokens are purged by hourly maintenance.
- Rate limits per 15 minutes: requests, 3 per address and 10 per IP; reset attempts, 10 per IP.

## Testing

- Database: single use, expiry, newest link wins, purge, concurrent use (one succeeds).
- API: identical reply for known and unknown addresses, rate limits, SSO-only accounts, admin route refused for non-admins.
- Web: forgot page, reset page, expired link, admin modal.
- Browser: the full flow through Mailpit on the throwaway stack, and the admin link flow with mail off.
