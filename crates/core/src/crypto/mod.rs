use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
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

/// A data key wrapped by a key derived from a secret.
pub struct WrappedKey {
    pub kdf_salt: Vec<u8>,
    pub wrapped_key: Vec<u8>,
    pub wrapped_nonce: Vec<u8>,
}

pub fn new_data_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// Wrap `data_key` under `secret`. `unlock_vault` reverses it.
pub fn wrap_data_key(secret: &str, data_key: &[u8; KEY_LEN]) -> Result<WrappedKey> {
    let kdf_salt = random_bytes(SALT_LEN);
    let wrapping_key = derive_key(secret, &kdf_salt)?;
    let (wrapped_key, wrapped_nonce) = encrypt(&wrapping_key, data_key)?;
    Ok(WrappedKey {
        kdf_salt,
        wrapped_key,
        wrapped_nonce,
    })
}

/// Wrap a vault's data key under the user's master key.
pub fn wrap_with_key(
    master: &[u8; KEY_LEN],
    data_key: &[u8; KEY_LEN],
) -> Result<(Vec<u8>, Vec<u8>)> {
    encrypt(master, data_key)
}

pub fn unwrap_with_key(
    master: &[u8; KEY_LEN],
    wrapped_key: &[u8],
    wrapped_nonce: &[u8],
) -> Result<[u8; KEY_LEN]> {
    let plain =
        decrypt(master, wrapped_key, wrapped_nonce).map_err(|_| Error::VaultUnlockFailed)?;
    plain.try_into().map_err(|_| Error::VaultUnlockFailed)
}

const RECOVERY_ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const RECOVERY_LEN: usize = 52;

/// 52 base32 characters (260 bits), grouped in fours for reading.
pub fn new_recovery_key() -> String {
    random_bytes(RECOVERY_LEN)
        .chunks(4)
        .map(|chunk| {
            chunk
                .iter()
                .map(|b| RECOVERY_ALPHABET[(*b & 31) as usize] as char)
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("-")
}

/// The form a recovery key is wrapped under, so case and spacing do not matter.
pub fn normalize_recovery_key(input: &str) -> String {
    input
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
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
    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|e| Error::msg(e.to_string()))?;
    let nonce_bytes = random_bytes(NONCE_LEN);
    let nonce: [u8; NONCE_LEN] = nonce_bytes
        .as_slice()
        .try_into()
        .map_err(|_| Error::msg("invalid nonce length"))?;
    let ciphertext = cipher
        .encrypt(&Nonce::from(nonce), plaintext)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok((ciphertext, nonce_bytes))
}

fn decrypt(key_bytes: &[u8], ciphertext: &[u8], nonce_bytes: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key_bytes).map_err(|_| Error::VaultUnlockFailed)?;
    let nonce: [u8; NONCE_LEN] = nonce_bytes
        .try_into()
        .map_err(|_| Error::VaultUnlockFailed)?;
    cipher
        .decrypt(&Nonce::from(nonce), ciphertext)
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
    fn vault_payload_encrypt_decrypt() {
        let data_key = new_data_key();
        let payload = VaultPayload {
            url: "https://example.com".into(),
            title: "Example".into(),
            description: Some("desc".into()),
            notes: None,
            tags: vec!["a".into()],
        };
        let (ct, nonce) = encrypt_payload(&data_key, &payload).unwrap();
        let back = decrypt_payload(&data_key, &ct, &nonce).unwrap();
        assert_eq!(back.url, payload.url);
        assert_eq!(back.title, payload.title);
        assert_eq!(back.tags, payload.tags);
    }

    #[test]
    fn decrypt_rejects_bad_key_or_nonce_length() {
        let bad_nonce = super::decrypt(&[0u8; super::KEY_LEN], b"x", &[0u8; 4]);
        assert!(matches!(bad_nonce, Err(crate::Error::VaultUnlockFailed)));
        let bad_key = super::decrypt(&[0u8; 4], b"x", &[0u8; super::NONCE_LEN]);
        assert!(matches!(bad_key, Err(crate::Error::VaultUnlockFailed)));
    }
}

#[cfg(test)]
mod key_wrap_tests {
    use super::*;

    #[test]
    fn a_secret_wrap_round_trips() {
        let key = [7u8; KEY_LEN];
        let wrapped = wrap_data_key("correct horse", &key).unwrap();
        let got = unlock_vault(
            "correct horse",
            &wrapped.kdf_salt,
            &wrapped.wrapped_key,
            &wrapped.wrapped_nonce,
        )
        .unwrap();
        assert_eq!(got, key);
    }

    #[test]
    fn a_wrong_secret_fails() {
        let wrapped = wrap_data_key("correct horse", &[7u8; KEY_LEN]).unwrap();
        let got = unlock_vault(
            "wrong horse",
            &wrapped.kdf_salt,
            &wrapped.wrapped_key,
            &wrapped.wrapped_nonce,
        );
        assert!(matches!(got, Err(Error::VaultUnlockFailed)));
    }

    #[test]
    fn a_master_key_wrap_round_trips() {
        let master = [1u8; KEY_LEN];
        let key = [2u8; KEY_LEN];
        let (ct, nonce) = wrap_with_key(&master, &key).unwrap();
        assert_eq!(unwrap_with_key(&master, &ct, &nonce).unwrap(), key);
        assert!(matches!(
            unwrap_with_key(&[3u8; KEY_LEN], &ct, &nonce),
            Err(Error::VaultUnlockFailed)
        ));
    }

    #[test]
    fn recovery_keys_are_long_and_normalize() {
        let code = new_recovery_key();
        assert_eq!(code.replace('-', "").len(), 52);
        assert_ne!(code, new_recovery_key());
        let messy = format!("  {} ", code.to_ascii_lowercase().replace('-', " "));
        assert_eq!(
            normalize_recovery_key(&messy),
            normalize_recovery_key(&code)
        );
    }
}
