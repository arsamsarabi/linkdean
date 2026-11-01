-- Profile and library visibility. Defaults keep every profile and library invisible.

ALTER TABLE users
    ADD COLUMN profile_visibility TEXT NOT NULL DEFAULT 'hidden'
    CONSTRAINT users_profile_visibility_check
    CHECK (profile_visibility IN ('members', 'followers', 'hidden'));

ALTER TABLE categories
    ADD COLUMN visibility TEXT NOT NULL DEFAULT 'private'
    CONSTRAINT categories_visibility_check
    CHECK (visibility IN ('private', 'followers', 'members'));

ALTER TABLE categories
    ADD CONSTRAINT categories_vault_visibility_check
    CHECK (NOT is_vault OR visibility = 'private');

ALTER TABLE bookmarks
    ADD COLUMN is_private BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX categories_owner_visibility_idx
    ON categories (owner_user_id, visibility)
    WHERE deleted_at IS NULL AND visibility <> 'private';
