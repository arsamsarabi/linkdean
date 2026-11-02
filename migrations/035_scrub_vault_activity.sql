-- Updates to vault bookmarks logged their decrypted titles. Vault rows carry a fixed title.
UPDATE activity_events
SET summary = 'Updated bookmark “Vault bookmark”', payload = '{}'::jsonb
WHERE subject_type = 'bookmark'
  AND action = 'updated'
  AND subject_title = 'Vault bookmark';
