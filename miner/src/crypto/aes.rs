use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce
};
use aes_gcm::aead::generic_array::GenericArray;
// use rand::RngCore;
use crate::error::Error;

/// Encrypts data using AES-256-GCM
/// 
/// # Arguments
/// * `data` - Plaintext data to encrypt
/// * `key` - 32-byte encryption key
/// 
/// # Returns
/// Combined ciphertext + nonce (nonce is last 12 bytes)
pub fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, Error> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::Custom(format!("Key initialization failed: {}", e)))?;
    
    // Generate random nonce
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let nonce = Nonce::from_slice(&nonce);
    
    cipher.encrypt(nonce, data)
        .map(|mut ciphertext| {
            ciphertext.extend_from_slice(nonce);
            ciphertext
        })
        .map_err(|e| Error::Custom(format!("Encryption failed: {}", e)))
}

/// Decrypts data using AES-256-GCM
/// 
/// # Arguments
/// * `data` - Ciphertext with appended nonce (last 12 bytes)
/// * `key` - 32-byte decryption key
/// 
/// # Returns
/// Decrypted plaintext data
pub fn decrypt(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, Error> {
    if data.len() < 12 {
        return Err(Error::Custom("Ciphertext too short".into()));
    }
    
    let (ciphertext, nonce) = data.split_at(data.len() - 12);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| Error::Custom(format!("Key initialization failed: {}", e)))?;
    let nonce = GenericArray::from_slice(nonce);
    cipher.decrypt(nonce, ciphertext)
        .map_err(|e| Error::Custom(format!("Decryption failed: {}", e)))
}