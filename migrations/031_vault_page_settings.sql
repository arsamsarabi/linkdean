-- Whether unlocked vault items also appear in the Library, and whether deleting
-- a vault item sends it to Trash or removes it immediately.

ALTER TABLE user_settings
    ADD COLUMN show_vault_in_library BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN vault_delete_mode TEXT NOT NULL DEFAULT 'trash'
        CHECK (vault_delete_mode IN ('trash', 'permanent'));
