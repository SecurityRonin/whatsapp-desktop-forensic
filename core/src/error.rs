//! Typed, fail-loud errors — a wrong key or a bad read surfaces as a named
//! variant with context, never a swallowed default or fabricated value.

use thiserror::Error;

/// An error reading or decrypting a WhatsApp Desktop store.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WaError {
    /// The underlying LevelDB store could not be read.
    #[error("LevelDB read failed: {0}")]
    LevelDb(#[from] leveldb_core::Error),
    /// AES-CBC decryption failed — wrong key or corrupt ciphertext. The plaintext
    /// is **not** produced (no fabrication); the caller must treat the body as
    /// undecryptable.
    #[error("message body decryption failed: {0}")]
    Decrypt(&'static str),
    /// The supplied AES key length is not 16 (AES-128) or 32 (AES-256) bytes.
    #[error("unsupported AES key length {0} bytes (expected 16 or 32)")]
    BadKeyLen(usize),
    /// The supplied AES-CBC IV is not 16 bytes.
    #[error("invalid AES-CBC IV length {0} bytes (expected 16)")]
    BadIvLen(usize),
}
