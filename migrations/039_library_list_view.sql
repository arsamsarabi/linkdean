-- A compact list layout for the library, beside grid and explorer.
ALTER TABLE user_settings DROP CONSTRAINT IF EXISTS user_settings_library_view_check;
ALTER TABLE user_settings
    ADD CONSTRAINT user_settings_library_view_check
        CHECK (library_view IN ('grid', 'list', 'explorer'));
