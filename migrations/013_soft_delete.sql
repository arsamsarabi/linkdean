-- Soft-delete / trash for bookmarks, categories, dashboards + retention prefs.

ALTER TABLE bookmarks
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

ALTER TABLE categories
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

ALTER TABLE dashboards
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

CREATE INDEX IF NOT EXISTS bookmarks_owner_deleted_idx
    ON bookmarks (owner_user_id, deleted_at);

CREATE INDEX IF NOT EXISTS categories_owner_deleted_idx
    ON categories (owner_user_id, deleted_at);

CREATE INDEX IF NOT EXISTS dashboards_owner_deleted_idx
    ON dashboards (owner_user_id, deleted_at);

ALTER TABLE user_settings
    ADD COLUMN IF NOT EXISTS trash_retention_days INT NOT NULL DEFAULT 30
        CHECK (trash_retention_days > 0),
    ADD COLUMN IF NOT EXISTS show_deleted_in_library BOOLEAN NOT NULL DEFAULT FALSE;
