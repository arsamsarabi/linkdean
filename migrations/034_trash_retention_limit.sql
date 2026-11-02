-- The purge subtracts this many days from now, and a very large value panics at startup.
UPDATE user_settings SET trash_retention_days = 3650 WHERE trash_retention_days > 3650;

ALTER TABLE user_settings DROP CONSTRAINT user_settings_trash_retention_days_check;
ALTER TABLE user_settings
    ADD CONSTRAINT user_settings_trash_retention_days_check
    CHECK (trash_retention_days BETWEEN 1 AND 3650);
