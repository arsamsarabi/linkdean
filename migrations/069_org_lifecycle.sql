-- Orgs Phase A slice 5: team soft-delete + indexes for lifecycle / audit filters.

ALTER TABLE teams
ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS teams_org_deleted_idx ON teams (organization_id, deleted_at)
WHERE
    deleted_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS teams_org_archived_idx ON teams (organization_id)
WHERE
    archived_at IS NOT NULL
    AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS organization_audit_events_org_actor_idx ON organization_audit_events (organization_id, actor_user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS organization_audit_events_org_action_idx ON organization_audit_events (organization_id, action, created_at DESC);

CREATE INDEX IF NOT EXISTS organizations_deleted_idx ON organizations (deleted_at)
WHERE
    deleted_at IS NOT NULL;
