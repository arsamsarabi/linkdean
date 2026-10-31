use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::RngCore;

use crate::{Error, Result, VaultPayload};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| Error::msg(e.to_string()))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed = PasswordHash::new(hash).map_err(|e| Error::msg(e.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

pub struct VaultKeys {
    pub kdf_salt: Vec<u8>,
    pub data_key: [u8; KEY_LEN],
    pub wrapped_key: Vec<u8>,
    pub wrapped_nonce: Vec<u8>,
}

pub fn setup_vault(passphrase: &str) -> Result<VaultKeys> {
    let kdf_salt = random_bytes(SALT_LEN);
    let wrapping_key = derive_key(passphrase, &kdf_salt)?;
    let mut data_key = [0u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut data_key);
    let (wrapped_key, wrapped_nonce) = encrypt(&wrapping_key, &data_key)?;
    Ok(VaultKeys {
        kdf_salt,
        data_key,
        wrapped_key,
        wrapped_nonce,
    })
}

pub fn unlock_vault(
    passphrase: &str,
    kdf_salt: &[u8],
    wrapped_key: &[u8],
    wrapped_nonce: &[u8],
) -> Result<[u8; KEY_LEN]> {
    let wrapping_key = derive_key(passphrase, kdf_salt)?;
    let plain =
        decrypt(&wrapping_key, wrapped_key, wrapped_nonce).map_err(|_| Error::VaultUnlockFailed)?;
    if plain.len() != KEY_LEN {
        return Err(Error::VaultUnlockFailed);
    }
    let mut key = [0u8; KEY_LEN];
    key.copy_from_slice(&plain);
    Ok(key)
}

pub fn encrypt_payload(
    data_key: &[u8; KEY_LEN],
    payload: &VaultPayload,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let bytes = serde_json::to_vec(payload).map_err(|e| Error::msg(e.to_string()))?;
    encrypt(data_key, &bytes)
}

pub fn decrypt_payload(
    data_key: &[u8; KEY_LEN],
    ciphertext: &[u8],
    nonce: &[u8],
) -> Result<VaultPayload> {
    let plain = decrypt(data_key, ciphertext, nonce)?;
    serde_json::from_slice(&plain).map_err(|e| Error::msg(e.to_string()))
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; KEY_LEN]> {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(key)
}

fn encrypt(key_bytes: &[u8], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes = random_bytes(NONCE_LEN);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok((ciphertext, nonce_bytes))
}

fn decrypt(key_bytes: &[u8], ciphertext: &[u8], nonce_bytes: &[u8]) -> Result<Vec<u8>> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| Error::VaultUnlockFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VaultPayload;

    #[test]
    fn password_hash_round_trip() {
        let hash = hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &hash).unwrap());
        assert!(!verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn token_hash_stable() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }

    #[test]
    fn vault_setup_unlock_round_trip() {
        let keys = setup_vault("passphrase-long").unwrap();
        let unlocked = unlock_vault(
            "passphrase-long",
            &keys.kdf_salt,
            &keys.wrapped_key,
            &keys.wrapped_nonce,
        )
        .unwrap();
        assert_eq!(unlocked, keys.data_key);
        assert!(matches!(
            unlock_vault(
                "wrong",
                &keys.kdf_salt,
                &keys.wrapped_key,
                &keys.wrapped_nonce,
            ),
            Err(Error::VaultUnlockFailed)
        ));
    }

    #[test]
    fn vault_payload_encrypt_decrypt() {
        let keys = setup_vault("passphrase-long").unwrap();
        let payload = VaultPayload {
            url: "https://example.com".into(),
            title: "Example".into(),
            description: Some("desc".into()),
            notes: None,
            tags: vec!["a".into()],
        };
        let (ct, nonce) = encrypt_payload(&keys.data_key, &payload).unwrap();
        let back = decrypt_payload(&keys.data_key, &ct, &nonce).unwrap();
        assert_eq!(back.url, payload.url);
        assert_eq!(back.title, payload.title);
        assert_eq!(back.tags, payload.tags);
    }
}
