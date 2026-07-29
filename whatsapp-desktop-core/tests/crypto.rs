//! AES-CBC message-body decryption — validated against **independent** openssl
//! `enc -aes-*-cbc` Known-Answer-Test vectors (PKCS7), not a self-encoded
//! round-trip. A wrong or ill-formed key must fail loud, never fabricate bytes.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use hex_literal::hex;
use whatsapp_desktop_core::{decrypt_body, WaError};

// openssl enc -aes-256-cbc -K 0102..20 -iv 0001..0f -nosalt, PKCS7.
const KEY256: [u8; 32] = hex!("0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20");
const KEY128: [u8; 16] = hex!("0102030405060708090a0b0c0d0e0f10");
const IV: [u8; 16] = hex!("000102030405060708090a0b0c0d0e0f");
const CT256: [u8; 48] =
    hex!("e52fc6172af8c0cba684baecb46594188c16f540d2402c394cb9409a6a9385e18c81d24a9ffcc80fdc32c694493c0297");
const CT128: [u8; 48] =
    hex!("1f0fe41723155396f55c9e2c0d1578e38001047dc347311a20ab653c4950ac337a10e35879a023fa7ddc859fabbae716");
const PLAINTEXT: &[u8] = b"WhatsApp E2E body plaintext, 42!";

#[test]
fn decrypts_aes256_cbc_kat() {
    let out = decrypt_body(&CT256, &IV, &KEY256).expect("KAT decrypts");
    assert_eq!(out, PLAINTEXT);
}

#[test]
fn decrypts_aes128_cbc_kat() {
    let out = decrypt_body(&CT128, &IV, &KEY128).expect("KAT decrypts");
    assert_eq!(out, PLAINTEXT);
}

#[test]
fn wrong_key_fails_loud_not_fabricated() {
    let mut bad = KEY256;
    bad[0] ^= 0xff;
    let r = decrypt_body(&CT256, &IV, &bad);
    // A wrong key corrupts PKCS7 padding -> typed error, never plausible bytes.
    assert!(matches!(r, Err(WaError::Decrypt(_))), "got {r:?}");
}

#[test]
fn wrong_aes128_key_fails_loud_not_fabricated() {
    // The AES-128 branch must fail loud on a wrong key exactly like AES-256:
    // corrupt a 16-byte key so PKCS7 unpadding rejects it -> typed error, never
    // plausible-but-wrong plaintext.
    let mut bad = KEY128;
    bad[0] ^= 0xff;
    let r = decrypt_body(&CT128, &IV, &bad);
    assert!(matches!(r, Err(WaError::Decrypt(_))), "got {r:?}");
}

#[test]
fn bad_key_length_fails_loud() {
    let r = decrypt_body(&CT256, &IV, &[0u8; 20]);
    assert!(matches!(r, Err(WaError::BadKeyLen(20))), "got {r:?}");
}

#[test]
fn bad_iv_length_fails_loud() {
    let r = decrypt_body(&CT256, &[0u8; 8], &KEY256);
    assert!(matches!(r, Err(WaError::BadIvLen(8))), "got {r:?}");
}
