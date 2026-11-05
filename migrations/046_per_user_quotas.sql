-- Platform-default per-user quotas (NULL = unlimited) and period-keyed counters.

ALTER TABLE platform_settings
    ADD COLUMN IF NOT EXISTS max_bookmarks BIGINT,
    ADD COLUMN IF NOT EXISTS max_stored_image_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS max_enrich_actions_per_day BIGINT,
    ADD COLUMN IF NOT EXISTS max_import_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS max_upload_image_bytes BIGINT;

ALTER TABLE platform_settings DROP CONSTRAINT IF EXISTS platform_settings_max_bookmarks_pos;
ALTER TABLE platform_settings DROP CONSTRAINT IF EXISTS platform_settings_max_stored_image_bytes_pos;
ALTER TABLE platform_settings DROP CONSTRAINT IF EXISTS platform_settings_max_enrich_actions_per_day_pos;
ALTER TABLE platform_settings DROP CONSTRAINT IF EXISTS platform_settings_max_import_bytes_pos;
ALTER TABLE platform_settings DROP CONSTRAINT IF EXISTS platform_settings_max_upload_image_bytes_pos;

ALTER TABLE platform_settings
    ADD CONSTRAINT platform_settings_max_bookmarks_pos
        CHECK (max_bookmarks IS NULL OR max_bookmarks > 0),
    ADD CONSTRAINT platform_settings_max_stored_image_bytes_pos
        CHECK (max_stored_image_bytes IS NULL OR max_stored_image_bytes > 0),
    ADD CONSTRAINT platform_settings_max_enrich_actions_per_day_pos
        CHECK (max_enrich_actions_per_day IS NULL OR max_enrich_actions_per_day > 0),
    ADD CONSTRAINT platform_settings_max_import_bytes_pos
        CHECK (max_import_bytes IS NULL OR max_import_bytes > 0),
    ADD CONSTRAINT platform_settings_max_upload_image_bytes_pos
        CHECK (max_upload_image_bytes IS NULL OR max_upload_image_bytes > 0);

CREATE TABLE IF NOT EXISTS user_quota_usage (
    user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    meter TEXT NOT NULL,
    period_start DATE NOT NULL,
    count BIGINT NOT NULL DEFAULT 0 CHECK (count >= 0),
    PRIMARY KEY (user_id, meter, period_start)
);

CREATE INDEX IF NOT EXISTS user_quota_usage_period_idx
    ON user_quota_usage (period_start);
