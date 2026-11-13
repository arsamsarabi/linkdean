-- Whether the app shows keyboard hints: key numbers on dashboard bookmarks, key hint lines and
-- shortcut keycaps. The keys work either way; this only hides the visual cues.

ALTER TABLE user_settings
    ADD COLUMN show_keyboard_hints BOOLEAN NOT NULL DEFAULT TRUE;
