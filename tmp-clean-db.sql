-- User-requested cleanup: keep arsamsarabi@me.com + qa-alice@example.com only.
BEGIN;

DELETE FROM users
WHERE email NOT IN (
  'arsamsarabi@me.com',
  'qa-alice@example.com'
);

DELETE FROM pending_share_invites
WHERE invited_by_user_id NOT IN (SELECT id FROM users)
   OR (claimed_by_user_id IS NOT NULL AND claimed_by_user_id NOT IN (SELECT id FROM users));

DELETE FROM registration_invites
WHERE used_at IS NOT NULL
   OR revoked_at IS NOT NULL
   OR expires_at < now()
   OR created_by_user_id NOT IN (SELECT id FROM users);

COMMIT;

SELECT id, email, display_name, is_admin FROM users ORDER BY created_at;

SELECT
  (SELECT count(*) FROM bookmarks) AS bookmarks,
  (SELECT count(*) FROM categories) AS categories,
  (SELECT count(*) FROM dashboards) AS dashboards,
  (SELECT count(*) FROM category_shares) AS category_shares,
  (SELECT count(*) FROM pending_share_invites) AS pending_invites,
  (SELECT count(*) FROM activity_events) AS activity_events,
  (SELECT count(*) FROM notifications) AS notifications;
