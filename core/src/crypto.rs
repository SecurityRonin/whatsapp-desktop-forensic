//! WhatsApp Web message-body decryption — **audited RustCrypto AES-CBC only**.
//!
//! WhatsApp Web AES-CBC-encrypts a message's text body into
//! `msgRowOpaqueData:{_keyId, iv, _data}`. Given the derived AES key, this module
//! decrypts `_data` with the [RustCrypto](https://github.com/RustCrypto) `aes` +
//! `cbc` crates (PKCS7). It is **never** hand-rolled and **never** a placeholder:
//! a wrong or ill-formed key fails loud with a typed [`WaError`], it does not
//! return plausible-but-wrong bytes.
//!
//! Deriving the key itself (HKDF-SHA256 over a master key in IndexedDB
//! `wawc_db_enc` + the `WebEncKeySalt` from localStorage) is **out of scope** for
//! this crate: no real key material was available to validate the derivation
//! wiring against an oracle, and shipping unverified crypto glue is exactly the
//! failure this discipline forbids. Callers supply the derived key; see
//! `docs/validation.md`.

use crate::error::WaError;
use aes::{Aes128, Aes256};
use cbc::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};

type Aes128CbcDec = cbc::Decryptor<Aes128>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

/// Decrypt a WhatsApp Web message body (`msgRowOpaqueData._data`).
///
/// `key` must be 16 (AES-128) or 32 (AES-256) bytes; `iv` must be 16 bytes.
///
/// # Errors
///
/// - [`WaError::BadIvLen`] / [`WaError::BadKeyLen`] for a mis-sized IV/key.
/// - [`WaError::Decrypt`] when PKCS7 unpadding fails (wrong key or corrupt
///   ciphertext) — the function returns an error rather than fabricating bytes.
pub fn decrypt_body(ciphertext: &[u8], iv: &[u8], key: &[u8]) -> Result<Vec<u8>, WaError> {
    if iv.len() != 16 {
        return Err(WaError::BadIvLen(iv.len()));
    }
    let unpad_err = || WaError::Decrypt("PKCS7 unpad failed (wrong key or corrupt ciphertext)");
    match key.len() {
        16 => Aes128CbcDec::new_from_slices(key, iv)
            .map_err(|_| WaError::Decrypt("invalid AES-128 key/iv"))?
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| unpad_err()),
        32 => Aes256CbcDec::new_from_slices(key, iv)
            .map_err(|_| WaError::Decrypt("invalid AES-256 key/iv"))?
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| unpad_err()),
        n => Err(WaError::BadKeyLen(n)),
    }
}
