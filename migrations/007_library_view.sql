ALTER TABLE user_settings
    ADD COLUMN IF NOT EXISTS library_view TEXT NOT NULL DEFAULT 'grid'
        CHECK (library_view IN ('grid', 'explorer'));
