-- Organizations Phase C2: join-on-request for verified domains.

CREATE TABLE IF NOT EXISTS organization_domain_join_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    domain_id UUID NOT NULL REFERENCES organization_domains (id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ,
    resolved_by_user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    CONSTRAINT organization_domain_join_requests_status_check CHECK (
        status IN ('pending', 'approved', 'denied')
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS organization_domain_join_requests_pending_uidx ON organization_domain_join_requests (organization_id, user_id)
WHERE
    status = 'pending';

CREATE INDEX IF NOT EXISTS organization_domain_join_requests_org_pending_idx ON organization_domain_join_requests (organization_id, created_at DESC)
WHERE
    status = 'pending';
