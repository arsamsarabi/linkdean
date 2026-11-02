-- Likes, comments and ratings point at their subject by type and id, with no foreign key.
-- Hard-deleting a bookmark, list or dashboard left them behind. A trigger removes them on
-- every delete path, including cascades from a deleted account.

CREATE OR REPLACE FUNCTION delete_social_for_subject() RETURNS trigger AS $$
BEGIN
    DELETE FROM social_likes WHERE subject_type = TG_ARGV[0] AND subject_id = OLD.id;
    DELETE FROM social_comments WHERE subject_type = TG_ARGV[0] AND subject_id = OLD.id;
    DELETE FROM social_ratings WHERE subject_type = TG_ARGV[0] AND subject_id = OLD.id;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS bookmarks_delete_social ON bookmarks;
CREATE TRIGGER bookmarks_delete_social AFTER DELETE ON bookmarks
    FOR EACH ROW EXECUTE FUNCTION delete_social_for_subject('bookmark');

DROP TRIGGER IF EXISTS categories_delete_social ON categories;
CREATE TRIGGER categories_delete_social AFTER DELETE ON categories
    FOR EACH ROW EXECUTE FUNCTION delete_social_for_subject('category');

DROP TRIGGER IF EXISTS dashboards_delete_social ON dashboards;
CREATE TRIGGER dashboards_delete_social AFTER DELETE ON dashboards
    FOR EACH ROW EXECUTE FUNCTION delete_social_for_subject('dashboard');

-- Rows already orphaned by earlier deletes.
DELETE FROM social_likes s WHERE NOT EXISTS (
    SELECT 1 FROM bookmarks WHERE s.subject_type = 'bookmark' AND id = s.subject_id
    UNION ALL SELECT 1 FROM categories WHERE s.subject_type = 'category' AND id = s.subject_id
    UNION ALL SELECT 1 FROM dashboards WHERE s.subject_type = 'dashboard' AND id = s.subject_id
);
DELETE FROM social_comments s WHERE NOT EXISTS (
    SELECT 1 FROM bookmarks WHERE s.subject_type = 'bookmark' AND id = s.subject_id
    UNION ALL SELECT 1 FROM categories WHERE s.subject_type = 'category' AND id = s.subject_id
    UNION ALL SELECT 1 FROM dashboards WHERE s.subject_type = 'dashboard' AND id = s.subject_id
);
DELETE FROM social_ratings s WHERE NOT EXISTS (
    SELECT 1 FROM bookmarks WHERE s.subject_type = 'bookmark' AND id = s.subject_id
    UNION ALL SELECT 1 FROM categories WHERE s.subject_type = 'category' AND id = s.subject_id
    UNION ALL SELECT 1 FROM dashboards WHERE s.subject_type = 'dashboard' AND id = s.subject_id
);
