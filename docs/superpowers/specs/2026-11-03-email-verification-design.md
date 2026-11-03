# Outbound email, part C: email verification

Approved design, not yet planned. Builds on part A (mail foundation) and part B (password reset, whose token table and fragment-link pattern this reuses). It closes the known issue on `create_share` in `crates/db/src/repo/shares.rs`: today a share to an address that has an account goes straight to that account, whether or not its owner controls the address.

## Gates

- **Soft gate (default).** Unverified accounts use the whole app. Their address cannot receive invites (below). When mail is on, a banner says "Verify your email" with a Resend button. With mail off nobody can verify, so the banner is not shown.
- **Hard gate (instance setting).** Admin → Platform → "Require verified email". It can only be turned on while mail is on. Unverified users then see a `/verify-email` screen with Resend and Sign out, and the API answers `403` with code `email_unverified` to every route except `/api/auth/*`, resend and logout.

## Verification link

- Sent at registration (password accounts) and from Resend.
- The link opens `/verify-email#token=…`. The token is in the fragment, as with password reset.
- Tokens are 32 random bytes, stored as SHA-256 hashes, valid 24 hours, single use. A new link voids the previous one. Stored in the part B token table with a `purpose` column (`password_reset`, `verify_email`, `change_email`), plus the target address for email change.
- Opening the link verifies the address without being signed in; the token proves control of the inbox. The page says "Email verified" and offers sign-in (or continues to the library when signed in).
- Rate limits per 15 minutes: Resend 3 per account and 10 per IP; verify attempts 10 per IP.
- OIDC accounts are verified when the provider reports `email_verified`.
- Email `verify_email`: a "Verify your email" button and the printed link.

## Existing accounts

- Every account that exists when this ships counts as verified.
- The upgrade note says so, and that an instance admin can remove verification from any account they do not trust.

## Invites to unverified accounts

- A folder or dashboard share to an address whose account is unverified becomes a pending invite for the address, not a direct share. The owner sees "Waiting for x@… to verify their email".
- When the account verifies, its pending invites attach automatically and still need accepting, as pending invites do today (`link_pending_share_invites`).
- With mail off, the claim link works as today and now also works while signed in, when the account's address matches the invite's.

## Email change

- Settings → Account → Change email. Requires the current password. Accounts without a local password (SSO only) cannot change their address here; their identity provider owns it.
- The link goes to the new address. The old address stays in use until the link is opened. Valid 24 hours; a newer request voids the older one.
- The new address must not belong to another account, checked again when the link is opened.
- The old address gets `email_change_notice`: "Your email is being changed to n•••@example.com. Not you? Reset your password." The new address gets `email_change_verify`.
- After the switch the account is verified with the new address, and pending invites for the new address attach.
- With mail off the option says "Ask your admin".

## Admin

- Accounts list: a Verified badge; actions Mark verified, Remove verification, and Change email (applied at once, no verification: the admin vouches for the address).
- Removing verification does not remove shares already accepted; it only routes new invites to pending.

## Out of scope

- Approved account linking for OIDC sign-ins (part D).
- Emailing invites and notifications (part E).

## Testing

- Database: single use, expiry, newest wins, purpose separation; verifying attaches pending invites; email change switches only when the link is opened and rechecks uniqueness.
- API: rate limits; a share to an unverified account becomes pending; the hard gate's 403 and its exceptions; turning the hard gate on is refused with mail off; admin actions refused for non-admins.
- Web: banner, verify page, change email form, hard-gate screen, admin actions.
- Browser: register → verify through Mailpit; change email; hard gate on; mail off (no banner, signed-in claim link).
