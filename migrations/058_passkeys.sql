-- Passkeys (WebAuthn discoverable credentials) that sign in to an account.
CREATE TABLE passkeys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- The authenticator's credential ID. One credential belongs to one account.
    credential_id BYTEA NOT NULL UNIQUE,
    name TEXT NOT NULL,
    -- The credential as webauthn-rs serializes it: public key, counter, backup state.
    passkey JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX passkeys_user ON passkeys (user_id);

-- A WebAuthn ceremony in progress: the challenge state the browser's answer is checked
-- against. Single use and short-lived.
CREATE TABLE webauthn_challenges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Set for adding a passkey; NULL for signing in, where the account is not known yet.
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    purpose TEXT NOT NULL CHECK (purpose IN ('register', 'login')),
    state JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX webauthn_challenges_expires ON webauthn_challenges (expires_at);
