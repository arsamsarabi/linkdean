-- Joining the global passphrase now seals the vault key to a public key, so it
-- needs no passphrase. Existing master keys have no public key on record and it
-- can only be derived with the passphrase, so the global passphrase is reset.
-- Vault passphrases and contents are untouched.

UPDATE vaults SET global_wrapped_key = NULL;
DELETE FROM vault_master_keys;

ALTER TABLE vaults DROP COLUMN global_wrapped_nonce;
ALTER TABLE vault_master_keys ADD COLUMN public_key BYTEA NOT NULL;
