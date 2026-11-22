-- Restricted folders in team spaces (Orgs Phase A slice 4).
-- Restrictions only narrow access: allowlisted members of the owning team still
-- get at most their team role. Org owners/admins (effective Maintainers) always see.
-- Personal folders cannot be restricted.

ALTER TABLE categories
    ADD COLUMN IF NOT EXISTS is_restricted BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE categories
    DROP CONSTRAINT IF EXISTS categories_restricted_team_only;
ALTER TABLE categories
    ADD CONSTRAINT categories_restricted_team_only CHECK (
        NOT is_restricted OR team_id IS NOT NULL
    );

CREATE TABLE IF NOT EXISTS category_restriction_members (
    category_id UUID NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    PRIMARY KEY (category_id, user_id)
);

CREATE INDEX IF NOT EXISTS category_restriction_members_user_idx
    ON category_restriction_members (user_id);
