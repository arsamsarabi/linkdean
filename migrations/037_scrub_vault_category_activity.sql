-- Activity for folders inside a vault logged their names. Vault folders now log a fixed title.
UPDATE activity_events e
SET subject_title = 'Vault folder',
    summary = CASE e.action
        WHEN 'created' THEN 'Created list: Vault folder'
        WHEN 'deleted' THEN 'Deleted list: Vault folder'
        WHEN 'restored' THEN 'Restored list: Vault folder'
        ELSE 'Updated list: Vault folder'
    END,
    payload = '{}'::jsonb
FROM categories c
WHERE e.subject_type = 'category'
  AND e.subject_id = c.id
  AND c.is_vault = TRUE;
