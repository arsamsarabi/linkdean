-- Unlisted public bookmark-list URLs (secret slug; anyone with link can read).
-- Null slug = not public. Vault categories must never have a slug (enforced in app).

ALTER TABLE categories
    ADD COLUMN public_slug TEXT;

CREATE UNIQUE INDEX categories_public_slug_uidx
    ON categories (public_slug)
    WHERE public_slug IS NOT NULL;
