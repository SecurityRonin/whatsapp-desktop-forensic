//! Fuzz the AES-CBC body decryptor on arbitrary ciphertext/iv/key.
//!
//! Must never panic — every mis-sized key/iv and every corrupt ciphertext must
//! return a typed error, never crash and never fabricate plaintext.
#![no_main]
use libfuzzer_sys::fuzz_target;

use whatsapp_desktop_core::decrypt_body;

fuzz_target!(|input: (Vec<u8>, Vec<u8>, Vec<u8>)| {
    let (ciphertext, iv, key) = input;
    let _ = decrypt_body(&ciphertext, &iv, &key);
});
