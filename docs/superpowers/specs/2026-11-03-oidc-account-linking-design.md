# Outbound email, part D: linking single sign-on to existing accounts

Approved design, not yet planned. Closes the known issue in `crates/api/src/routes/auth/oidc.rs`: today an OIDC sign-in whose address already belongs to a password account is refused ("email already registered; sign in with your password"), so whoever registers an address with a password first blocks its owner's single sign-on. Builds on part C (verified addresses, admin email change) and part A (mail).

## Linking at sign-in

- When the OIDC callback finds no account for the provider identity (issuer + subject) but a password account with the same address (case-insensitive), it does not refuse. It stores a **pending link** and redirects to `/login?link=1`.
- A pending link holds the issuer, subject, provider email and whether the provider verified it. It lives on the server for 10 minutes and is bound to the browser by a one-time token in an HttpOnly, SameSite=Lax cookie. Nothing identifying goes in the URL. Only the token's hash is stored.
- The login page then shows: "An account already uses x@… Sign in with its password to connect single sign-on." (The address shown is the provider's, masked.)
- A successful password sign-in with the pending-link cookie attaches the identity to that account, consumes the pending link, and shows "Single sign-on connected".
- Nothing is linked when the password is wrong, the link expired or was used, or the password sign-in is for a different account than the one that holds the provider's address. The existing login rate limits apply.
- The account's address gets an email `sso_connected`: "Single sign-on ({{ provider }}) was connected to your account. Not you? Reset your password."

## Settings → Account → Single sign-on

- Shown when OIDC is configured. States: "Connected" (with the issuer host) or a **Connect** button.
- **Connect** starts the OIDC flow in "connect" mode while signed in. The callback attaches the identity to the signed-in account even when the provider's address differs; the account's address does not change.
- **Disconnect** needs the current password and is only offered to accounts that have one, so nobody loses their only way to sign in.

## Rules

- A provider identity belongs to at most one account. Linking an identity already attached elsewhere is refused: "This sign-in is already connected to another account."
- An account has at most one OIDC identity (the current `oidc_issuer` + `oidc_subject` columns).
- If the address belongs to an unverified password account and the person does not know its password, the login page says "Ask your admin". An instance admin can change that account's address (part C), which frees it for a normal SSO sign-in.
- Linking marks the account verified only when the provider's address equals the account's and the provider reports it verified.
- An existing OIDC sign-in with a matching identity is unchanged.

## Out of scope

- Setting a password on an SSO-only account.
- More than one identity provider per account.

## Testing

- Database: a pending link expires, works once, and only with its token; one identity per account and per subject.
- API: callback with an address held by a password account → pending link and redirect; password sign-in with the cookie links; wrong password, expiry, or a different account link nothing; Connect from settings; Disconnect needs the password and a local password; `sso_connected` email.
- Web: login link prompt, settings section.
- Browser: a throwaway mock OIDC provider (`ghcr.io/navikt/mock-oauth2-server`, pinned) against the throwaway stack: link at sign-in, connect from settings, disconnect.
