//! Save file encryption using AES-256-GCM
//!
//! Provides secure encryption for game save files with:
//! - AES-256-GCM authenticated encryption
//! - PBKDF2 key derivation (100k iterations)
//! - Per-save unique salt and nonce
//! - Tamper detection via GCM authentication tag

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::rngs::OsRng;
use rand::RngCore;
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;
use thiserror::Error;

/// Encryption errors
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    Encryption(String),
    #[error("Decryption failed: {0}")]
    Decryption(String),
    #[error("Invalid password")]
    InvalidPassword,
    #[error("Corrupted save file")]
    Corrupted,
    #[error("Unsupported version")]
    UnsupportedVersion,
}

/// Encryption version for future compatibility
const CURRENT_VERSION: u8 = 1;

/// PBKDF2 iterations (100k is OWASP recommended minimum)
const PBKDF2_ITERATIONS: u32 = 100_000;

/// Salt length (256 bits)
const SALT_LENGTH: usize = 32;

/// Nonce length (96 bits for GCM)
const NONCE_LENGTH: usize = 12;

/// Key length (256 bits)
const KEY_LENGTH: usize = 32;

/// Encrypted save file format
#[derive(Debug, Clone)]
pub struct EncryptedSave {
    /// Format version
    pub version: u8,
    /// PBKDF2 salt
    pub salt: [u8; SALT_LENGTH],
    /// AES-GCM nonce
    pub nonce: [u8; NONCE_LENGTH],
    /// Encrypted data
    pub ciphertext: Vec<u8>,
}

impl EncryptedSave {
    /// Create new encrypted save from plaintext
    pub fn encrypt(plaintext: &[u8], password: &str) -> Result<Self, EncryptionError> {
        // Generate random salt
        let mut salt = [0u8; SALT_LENGTH];
        OsRng.fill_bytes(&mut salt);

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        OsRng.fill_bytes(&mut nonce_bytes);

        // Derive key from password
        let key = derive_key(password, &salt);

        // Encrypt
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| EncryptionError::Encryption(e.to_string()))?;

        Ok(Self {
            version: CURRENT_VERSION,
            salt,
            nonce: nonce_bytes,
            ciphertext,
        })
    }

    /// Decrypt save file to plaintext
    pub fn decrypt(&self, password: &str) -> Result<Vec<u8>, EncryptionError> {
        if self.version != CURRENT_VERSION {
            return Err(EncryptionError::UnsupportedVersion);
        }

        // Derive key from password
        let key = derive_key(password, &self.salt);

        // Decrypt
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
        let nonce = Nonce::from_slice(&self.nonce);

        cipher
            .decrypt(nonce, self.ciphertext.as_ref())
            .map_err(|_| EncryptionError::InvalidPassword)
    }

    /// Serialize to bytes for storage
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Version (1 byte)
        result.push(self.version);

        // Salt (32 bytes)
        result.extend_from_slice(&self.salt);

        // Nonce (12 bytes)
        result.extend_from_slice(&self.nonce);

        // Ciphertext length (4 bytes, little-endian)
        result.extend_from_slice(&(self.ciphertext.len() as u32).to_le_bytes());

        // Ciphertext
        result.extend_from_slice(&self.ciphertext);

        result
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EncryptionError> {
        if bytes.len() < 1 + SALT_LENGTH + NONCE_LENGTH + 4 {
            return Err(EncryptionError::Corrupted);
        }

        let mut offset = 0;

        // Version
        let version = bytes[offset];
        offset += 1;

        // Salt
        let mut salt = [0u8; SALT_LENGTH];
        salt.copy_from_slice(&bytes[offset..offset + SALT_LENGTH]);
        offset += SALT_LENGTH;

        // Nonce
        let mut nonce = [0u8; NONCE_LENGTH];
        nonce.copy_from_slice(&bytes[offset..offset + NONCE_LENGTH]);
        offset += NONCE_LENGTH;

        // Ciphertext length
        let ct_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // Ciphertext
        if bytes.len() < offset + ct_len {
            return Err(EncryptionError::Corrupted);
        }
        let ciphertext = bytes[offset..offset + ct_len].to_vec();

        Ok(Self {
            version,
            salt,
            nonce,
            ciphertext,
        })
    }
}

/// Derive encryption key from password using PBKDF2
fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LENGTH] {
    let mut key = [0u8; KEY_LENGTH];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

/// Encrypt a game save string
pub fn encrypt_save(save_json: &str, password: &str) -> Result<Vec<u8>, EncryptionError> {
    let encrypted = EncryptedSave::encrypt(save_json.as_bytes(), password)?;
    Ok(encrypted.to_bytes())
}

/// Decrypt a game save to string
pub fn decrypt_save(encrypted_bytes: &[u8], password: &str) -> Result<String, EncryptionError> {
    let encrypted = EncryptedSave::from_bytes(encrypted_bytes)?;
    let plaintext = encrypted.decrypt(password)?;
    String::from_utf8(plaintext).map_err(|_| EncryptionError::Corrupted)
}

/// Check if a password is correct without full decryption
pub fn verify_password(encrypted_bytes: &[u8], password: &str) -> bool {
    match EncryptedSave::from_bytes(encrypted_bytes) {
        Ok(encrypted) => encrypted.decrypt(password).is_ok(),
        Err(_) => false,
    }
}

/// Generate a secure random password
pub fn generate_password() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789!@#$%^&*";
    let mut rng = OsRng;
    let mut password = String::with_capacity(32);

    for _ in 0..32 {
        let mut byte = [0u8; 1];
        rng.fill_bytes(&mut byte);
        let idx = (byte[0] as usize) % CHARSET.len();
        password.push(CHARSET[idx] as char);
    }

    password
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let plaintext = b"Hello, secure save world!";
        let password = "my_secure_password_123";

        // Encrypt
        let encrypted = EncryptedSave::encrypt(plaintext, password).unwrap();
        let bytes = encrypted.to_bytes();

        // Decrypt
        let loaded = EncryptedSave::from_bytes(&bytes).unwrap();
        let decrypted = loaded.decrypt(password).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_password() {
        let plaintext = b"Secret data";
        let encrypted = EncryptedSave::encrypt(plaintext, "correct_password").unwrap();

        // Wrong password should fail
        assert!(encrypted.decrypt("wrong_password").is_err());
    }

    #[test]
    fn test_verify_password() {
        let plaintext = b"Test data";
        let encrypted = EncryptedSave::encrypt(plaintext, "test_pass").unwrap();
        let bytes = encrypted.to_bytes();

        assert!(verify_password(&bytes, "test_pass"));
        assert!(!verify_password(&bytes, "wrong_pass"));
    }

    #[test]
    fn test_corrupted_data() {
        let plaintext = b"Important save data";
        let encrypted = EncryptedSave::encrypt(plaintext, "password").unwrap();
        let mut bytes = encrypted.to_bytes();

        // Corrupt some bytes
        if bytes.len() > 50 {
            bytes[50] ^= 0xFF;
        }

        // Should fail decryption
        let loaded = EncryptedSave::from_bytes(&bytes).unwrap();
        assert!(loaded.decrypt("password").is_err());
    }

    #[test]
    fn test_large_data() {
        let plaintext = vec![0xABu8; 10000];
        let password = "large_data_test";

        let encrypted = EncryptedSave::encrypt(&plaintext, password).unwrap();
        let decrypted = encrypted.decrypt(password).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_generate_password() {
        let pass1 = generate_password();
        let pass2 = generate_password();

        assert_eq!(pass1.len(), 32);
        assert_eq!(pass2.len(), 32);
        assert_ne!(pass1, pass2); // Should be random
    }

    #[test]
    fn test_save_json_roundtrip() {
        let save_json = r#"{
            "version": 1,
            "player_name": "Hero",
            "level": 42,
            "gold": 9999
        }"#;
        let password = "game_save_pass";

        // Encrypt
        let encrypted = encrypt_save(save_json, password).unwrap();

        // Decrypt
        let decrypted = decrypt_save(&encrypted, password).unwrap();

        assert_eq!(decrypted, save_json);
    }
}
