-- Organizations and teams (Phase A membership). Team-space content ownership comes later.
ALTER TABLE platform_settings
ADD COLUMN IF NOT EXISTS organizations_create_open BOOLEAN NOT NULL DEFAULT TRUE;

CREATE TABLE IF NOT EXISTS organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    CONSTRAINT organizations_name_len CHECK (
        char_length(name) >= 1
        AND char_length(name) <= 80
    )
);

CREATE TABLE IF NOT EXISTS organization_members (
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (organization_id, user_id),
    CONSTRAINT organization_members_role_check CHECK (
        role IN ('owner', 'admin', 'member', 'guest')
    )
);

CREATE INDEX IF NOT EXISTS organization_members_user_idx ON organization_members (user_id);

CREATE TABLE IF NOT EXISTS teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    archived_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT teams_name_len CHECK (
        char_length(name) >= 1
        AND char_length(name) <= 80
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS teams_one_default_per_org ON teams (organization_id)
WHERE
    is_default;

CREATE INDEX IF NOT EXISTS teams_organization_idx ON teams (organization_id);

CREATE TABLE IF NOT EXISTS team_members (
    team_id UUID NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (team_id, user_id),
    CONSTRAINT team_members_role_check CHECK (
        role IN ('maintainer', 'editor', 'viewer')
    )
);

CREATE INDEX IF NOT EXISTS team_members_user_idx ON team_members (user_id);

-- Append-only; app never updates or deletes except by age (later).
CREATE TABLE IF NOT EXISTS organization_audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    organization_id UUID NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
    actor_user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS organization_audit_events_org_created_idx ON organization_audit_events (
    organization_id,
    created_at DESC
);
