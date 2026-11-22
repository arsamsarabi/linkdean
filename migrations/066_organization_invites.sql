-- Organization email invites (Phase A slice 2). Accept grants membership; no access until then.
CREATE TABLE IF NOT EXISTS organization_invites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    invitee_email TEXT NOT NULL,
    role TEXT NOT NULL,
    invited_by_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    invitee_user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    claimed_at TIMESTAMPTZ,
    claimed_by_user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    revoked_at TIMESTAMPTZ,
    rejected_at TIMESTAMPTZ,
    CONSTRAINT organization_invites_role_check CHECK (
        role IN ('admin', 'member', 'guest')
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS organization_invites_active_uidx ON organization_invites (
    organization_id,
    lower(invitee_email)
)
WHERE
    claimed_at IS NULL
    AND revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS organization_invites_email_pending_idx ON organization_invites (lower(invitee_email))
WHERE
    claimed_at IS NULL
    AND revoked_at IS NULL;

CREATE INDEX IF NOT EXISTS organization_invites_invitee_user_pending_idx ON organization_invites (invitee_user_id)
WHERE
    claimed_at IS NULL
    AND revoked_at IS NULL
    AND invitee_user_id IS NOT NULL;

-- Teams the invitee should join on accept (guests need at least one; members may be empty → default team).
CREATE TABLE IF NOT EXISTS organization_invite_teams (
    invite_id UUID NOT NULL REFERENCES organization_invites (id) ON DELETE CASCADE,
    team_id UUID NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    team_role TEXT NOT NULL DEFAULT 'editor',
    PRIMARY KEY (invite_id, team_id),
    CONSTRAINT organization_invite_teams_role_check CHECK (
        team_role IN ('maintainer', 'editor', 'viewer')
    )
);
