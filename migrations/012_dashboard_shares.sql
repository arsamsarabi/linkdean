-- Milestone 3: share dashboards with same-instance users (read / write).

CREATE TABLE dashboard_shares (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dashboard_id UUID NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE,
    invitee_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access TEXT NOT NULL CHECK (access IN ('read', 'write')),
    invited_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (dashboard_id, invitee_user_id),
    CHECK (invitee_user_id <> invited_by_user_id)
);

CREATE INDEX dashboard_shares_invitee_idx ON dashboard_shares (invitee_user_id);
CREATE INDEX dashboard_shares_dashboard_idx ON dashboard_shares (dashboard_id);
