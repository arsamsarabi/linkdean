-- Organizations Phase D3: SCIM bearer tokens (one active per org).

CREATE TABLE IF NOT EXISTS organization_scim_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    prefix TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT organization_scim_tokens_name_len CHECK (
        char_length(name) >= 1
        AND char_length(name) <= 120
    )
);

CREATE INDEX IF NOT EXISTS organization_scim_tokens_org_active_idx ON organization_scim_tokens (organization_id, created_at DESC)
WHERE
    revoked_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS organization_scim_tokens_one_active_per_org ON organization_scim_tokens (organization_id)
WHERE
    revoked_at IS NULL;
