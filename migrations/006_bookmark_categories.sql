CREATE TABLE IF NOT EXISTS bookmark_categories (
    bookmark_id UUID NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (bookmark_id, category_id)
);

CREATE INDEX IF NOT EXISTS bookmark_categories_category_idx
    ON bookmark_categories (category_id);

INSERT INTO bookmark_categories (bookmark_id, category_id)
SELECT id, category_id FROM bookmarks
WHERE category_id IS NOT NULL
ON CONFLICT DO NOTHING;
