-- Optional icon key for categories (e.g. "star", "code"). Null = default folder.
ALTER TABLE categories
    ADD COLUMN icon TEXT;