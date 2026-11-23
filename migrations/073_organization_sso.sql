-- Organizations Phase D1: per-organization OIDC SSO config.

CREATE TABLE IF NOT EXISTS organization_sso_configs (
    organization_id UUID PRIMARY KEY REFERENCES organizations (id) ON DELETE CASCADE,
    issuer_url TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret_ciphertext BYTEA NOT NULL,
    client_secret_nonce BYTEA NOT NULL,
    display_name TEXT NOT NULL DEFAULT 'Company SSO',
    enabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_by_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS organization_sso_configs_enabled_idx ON organization_sso_configs (organization_id)
WHERE
    enabled;
