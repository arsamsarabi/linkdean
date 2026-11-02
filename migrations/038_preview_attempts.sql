-- A preview the owner deleted stays deleted, and a failed screenshot waits before retrying.
ALTER TABLE bookmarks
    ADD COLUMN preview_dismissed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN preview_failed_at TIMESTAMPTZ;
