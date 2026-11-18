-- Reader-mode page text for Meili body search (roadmap item 12). Meaningful only for
-- non-vault HTML rows; vault / file / pdf stay NULL.
ALTER TABLE bookmark_archives ADD COLUMN page_text TEXT;
