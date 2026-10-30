ALTER TABLE bookmarks
    ADD COLUMN IF NOT EXISTS preview_image_url TEXT;
