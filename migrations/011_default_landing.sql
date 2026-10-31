-- M2: default_landing is dashboard (/) or library (/library).
-- Legacy /vault and /import were never real app routes.

UPDATE user_settings
SET default_landing = '/'
WHERE default_landing IN ('/vault', '/import');

ALTER TABLE user_settings
    DROP CONSTRAINT IF EXISTS user_settings_default_landing_check;

ALTER TABLE user_settings
    ADD CONSTRAINT user_settings_default_landing_check
    CHECK (default_landing IN ('/', '/library'));
