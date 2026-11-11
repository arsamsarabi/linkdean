-- Retrying failed screenshots. An admin can retry them by hand at most once per cooldown,
-- and the automatic retry picks up failures older than the auto-retry window.
-- screenshot_retry_last_run_at is when the last manual retry started (NULL = never).
ALTER TABLE platform_settings
    ADD COLUMN screenshot_retry_cooldown_hours INTEGER NOT NULL DEFAULT 24
        CHECK (screenshot_retry_cooldown_hours BETWEEN 1 AND 720),
    ADD COLUMN screenshot_auto_retry_days INTEGER NOT NULL DEFAULT 7
        CHECK (screenshot_auto_retry_days BETWEEN 1 AND 90),
    ADD COLUMN screenshot_retry_last_run_at TIMESTAMPTZ;

CREATE INDEX bookmarks_preview_failed_idx ON bookmarks (preview_failed_at)
    WHERE preview_failed_at IS NOT NULL AND has_preview = FALSE AND deleted_at IS NULL;
