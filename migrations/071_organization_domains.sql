-- Organizations Phase C: verified domains (DNS TXT proof; join_mode for auto-join / request).

CREATE TABLE IF NOT EXISTS organization_domains (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    domain TEXT NOT NULL,
    verification_token TEXT NOT NULL,
    verified_at TIMESTAMPTZ,
    join_mode TEXT NOT NULL DEFAULT 'auto_join',
    created_by_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT organization_domains_domain_unique UNIQUE (domain),
    CONSTRAINT organization_domains_join_mode_check CHECK (
        join_mode IN ('auto_join', 'request')
    )
);

CREATE INDEX IF NOT EXISTS organization_domains_org_idx ON organization_domains (organization_id, created_at DESC);

CREATE UNIQUE INDEX IF NOT EXISTS organization_domains_verified_domain_uidx ON organization_domains (domain)
WHERE
    verified_at IS NOT NULL;
