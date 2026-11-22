-- Team-space content ownership (Orgs Phase A slice 3).
-- Personal rows keep owner_user_id; team-space rows use team_id (XOR).

-- categories
ALTER TABLE categories
ADD COLUMN IF NOT EXISTS team_id UUID REFERENCES teams (id) ON DELETE CASCADE,
ALTER COLUMN owner_user_id DROP NOT NULL;

ALTER TABLE categories
ADD CONSTRAINT categories_owner_xor_team CHECK (
    (
        owner_user_id IS NOT NULL
        AND team_id IS NULL
    )
    OR (
        owner_user_id IS NULL
        AND team_id IS NOT NULL
    )
);

ALTER TABLE categories
ADD CONSTRAINT categories_team_not_vault CHECK (
    team_id IS NULL
    OR (
        is_vault = FALSE
        AND vault_id IS NULL
    )
);

CREATE INDEX IF NOT EXISTS categories_team_idx ON categories (team_id)
WHERE
    team_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS categories_team_deleted_idx ON categories (team_id, deleted_at)
WHERE
    team_id IS NOT NULL;

-- bookmarks
ALTER TABLE bookmarks
ADD COLUMN IF NOT EXISTS team_id UUID REFERENCES teams (id) ON DELETE CASCADE,
ALTER COLUMN owner_user_id DROP NOT NULL;

ALTER TABLE bookmarks
ADD CONSTRAINT bookmarks_owner_xor_team CHECK (
    (
        owner_user_id IS NOT NULL
        AND team_id IS NULL
    )
    OR (
        owner_user_id IS NULL
        AND team_id IS NOT NULL
    )
);

ALTER TABLE bookmarks
ADD CONSTRAINT bookmarks_team_not_vault CHECK (
    team_id IS NULL
    OR (
        is_vault = FALSE
        AND vault_id IS NULL
    )
);

CREATE INDEX IF NOT EXISTS bookmarks_team_idx ON bookmarks (team_id)
WHERE
    team_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS bookmarks_team_deleted_idx ON bookmarks (team_id, deleted_at)
WHERE
    team_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS bookmarks_team_url_idx ON bookmarks (team_id, url)
WHERE
    team_id IS NOT NULL
    AND is_vault = FALSE;

-- tags (replace unique that assumes personal-only)
ALTER TABLE tags
ADD COLUMN IF NOT EXISTS team_id UUID REFERENCES teams (id) ON DELETE CASCADE,
ALTER COLUMN owner_user_id DROP NOT NULL;

ALTER TABLE tags DROP CONSTRAINT IF EXISTS tags_owner_user_id_name_key;

ALTER TABLE tags
ADD CONSTRAINT tags_owner_xor_team CHECK (
    (
        owner_user_id IS NOT NULL
        AND team_id IS NULL
    )
    OR (
        owner_user_id IS NULL
        AND team_id IS NOT NULL
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS tags_owner_name_unique ON tags (owner_user_id, name)
WHERE
    owner_user_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS tags_team_name_unique ON tags (team_id, name)
WHERE
    team_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS tags_team_idx ON tags (team_id)
WHERE
    team_id IS NOT NULL;

-- dashboards
ALTER TABLE dashboards
ADD COLUMN IF NOT EXISTS team_id UUID REFERENCES teams (id) ON DELETE CASCADE,
ALTER COLUMN owner_user_id DROP NOT NULL;

ALTER TABLE dashboards
ADD CONSTRAINT dashboards_owner_xor_team CHECK (
    (
        owner_user_id IS NOT NULL
        AND team_id IS NULL
    )
    OR (
        owner_user_id IS NULL
        AND team_id IS NOT NULL
    )
);

CREATE INDEX IF NOT EXISTS dashboards_team_idx ON dashboards (team_id)
WHERE
    team_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS dashboards_team_deleted_idx ON dashboards (team_id, deleted_at)
WHERE
    team_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS dashboards_one_default_per_team ON dashboards (team_id)
WHERE
    is_default
    AND team_id IS NOT NULL;
