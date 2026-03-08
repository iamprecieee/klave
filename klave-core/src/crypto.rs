use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit},
};
use rand::random;

use crate::error::{KlaveError, Result};

const NONCE_LEN: usize = 12;

pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

    let nonce_bytes: [u8; NONCE_LEN] = random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| KlaveError::Internal(format!("encryption failed: {e}")))?;

    let mut output = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

pub fn decrypt(blob: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if blob.len() < NONCE_LEN {
        return Err(KlaveError::Internal("encrypted blob too short".to_string()));
    }

    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| KlaveError::Internal(format!("decryption failed: {e}")))
}

pub fn parse_hex_key(hex: &str) -> Result<[u8; 32]> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return Err(KlaveError::Internal(format!(
            "KLAVE_ENCRYPTION_KEY must be 64 hex characters (32 bytes), got {}",
            hex.len()
        )));
    }

    let mut key = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let byte_str = std::str::from_utf8(chunk)
            .map_err(|_| KlaveError::Internal("invalid hex in encryption key".to_string()))?;
        key[i] = u8::from_str_radix(byte_str, 16)
            .map_err(|_| KlaveError::Internal("invalid hex in encryption key".to_string()))?;
    }
    Ok(key)
}
