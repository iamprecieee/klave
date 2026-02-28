use crate::crypto::{decrypt, encrypt, parse_hex_key};

fn test_key() -> [u8; 32] {
    [0xABu8; 32]
}

#[test]
fn roundtrip() {
    let plaintext = b"solana keypair bytes here";
    let key = test_key();
    let blob = encrypt(plaintext, &key).unwrap();
    let recovered = decrypt(&blob, &key).unwrap();
    assert_eq!(recovered, plaintext);
}

#[test]
fn wrong_key_fails() {
    let plaintext = b"secret";
    let blob = encrypt(plaintext, &test_key()).unwrap();
    let wrong_key = [0x00u8; 32];
    assert!(decrypt(&blob, &wrong_key).is_err());
}

#[test]
fn truncated_blob_fails() {
    assert!(decrypt(&[0u8; 5], &test_key()).is_err());
}

#[test]
fn parse_hex_key_valid() {
    let hex = "ab".repeat(32);
    let key = parse_hex_key(&hex).unwrap();
    assert_eq!(key, [0xABu8; 32]);
}

#[test]
fn parse_hex_key_wrong_length() {
    assert!(parse_hex_key("abcd").is_err());
}
