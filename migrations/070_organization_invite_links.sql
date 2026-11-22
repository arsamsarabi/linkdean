-- Organizations Phase B: shareable invite links (role, expiry, use limit; off by default).

ALTER TABLE organizations
ADD COLUMN IF NOT EXISTS invite_links_enabled BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS organization_invite_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    code_hash TEXT NOT NULL UNIQUE,
    code_prefix TEXT NOT NULL,
    role TEXT NOT NULL,
    created_by_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    max_uses INT,
    use_count INT NOT NULL DEFAULT 0,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT organization_invite_links_role_check CHECK (
        role IN ('admin', 'member', 'guest')
    ),
    CONSTRAINT organization_invite_links_max_uses_check CHECK (
        max_uses IS NULL
        OR max_uses > 0
    ),
    CONSTRAINT organization_invite_links_use_count_check CHECK (use_count >= 0)
);

CREATE INDEX IF NOT EXISTS organization_invite_links_org_active_idx ON organization_invite_links (organization_id, created_at DESC)
WHERE
    revoked_at IS NULL;

-- Teams the joiner should get when accepting (guests need at least one).
CREATE TABLE IF NOT EXISTS organization_invite_link_teams (
    link_id UUID NOT NULL REFERENCES organization_invite_links (id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    team_role TEXT NOT NULL DEFAULT 'editor',
    PRIMARY KEY (link_id, team_id),
    CONSTRAINT organization_invite_link_teams_role_check CHECK (
        team_role IN ('maintainer', 'editor', 'viewer')
    )
);
