ALTER TABLE users
    ADD COLUMN IF NOT EXISTS bio TEXT,
    ADD COLUMN IF NOT EXISTS timezone TEXT,
    ADD COLUMN IF NOT EXISTS has_avatar BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS user_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    show_hidden BOOLEAN NOT NULL DEFAULT FALSE,
    vault_search_mode TEXT NOT NULL DEFAULT 'when_unlocked'
        CHECK (vault_search_mode IN ('never', 'when_unlocked', 'always')),
    theme TEXT NOT NULL DEFAULT 'system'
        CHECK (theme IN ('light', 'dark', 'system')),
    vault_auto_lock_minutes INT NOT NULL DEFAULT 0
        CHECK (vault_auto_lock_minutes >= 0),
    default_landing TEXT NOT NULL DEFAULT '/'
        CHECK (default_landing IN ('/', '/vault', '/import')),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO user_settings (user_id)
SELECT id FROM users
ON CONFLICT (user_id) DO NOTHING;
