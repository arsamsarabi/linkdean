-- A single sign-on sign-in waiting for a password sign-in to attach it to an existing account.
-- The browser holds the token in a cookie; only its hash is stored. Users already have a unique
-- index on (oidc_issuer, oidc_subject) from 001_init, so an identity belongs to one account.
CREATE TABLE pending_oidc_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash TEXT NOT NULL UNIQUE,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    email TEXT NOT NULL,
    email_verified BOOLEAN NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
