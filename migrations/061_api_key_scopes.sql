-- Per-key permission scopes (library/archive/import_export/social/account × read/write).
ALTER TABLE api_keys
  ADD COLUMN scopes TEXT[] NOT NULL DEFAULT ARRAY[
    'library:read',
    'library:write',
    'archive:read',
    'archive:write',
    'import_export:read',
    'import_export:write',
    'social:read',
    'social:write',
    'account:read',
    'account:write'
  ]::text[];
