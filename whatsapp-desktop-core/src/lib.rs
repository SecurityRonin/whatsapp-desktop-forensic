//! WhatsApp Desktop (Electron) **reader** — interprets the Chromium
//! IndexedDB-over-LevelDB `model-storage` store into typed WhatsApp records.
//!
//! WhatsApp's Electron/PWA desktop client persists chats in a Chromium IndexedDB
//! database (`model-storage`) whose object-store values are Blink/V8
//! structured-clone blobs. This crate sits **on top of**
//! [`chromium_storage_indexeddb`] (which walks the LevelDB key-coding and decodes
//! each value's V8 graph) and maps those records into typed [`Message`],
//! [`Chat`], and [`Contact`] records plus a merged timeline.
//!
//! # The message body is encrypted — never fabricated
//!
//! A message's metadata is plaintext, but the text body is AES-CBC-encrypted in
//! `msgRowOpaqueData`. This reader surfaces that envelope as
//! [`MessageBody::Encrypted`] verbatim and **never** invents plaintext. Given the
//! derived key, [`crypto::decrypt_body`] performs the decryption via audited
//! RustCrypto AES-CBC; a wrong or missing key fails loud (a typed
//! [`WaError`]), it does not fabricate.
//!
//! Schema names are grounded in published WhatsApp Web reverse-engineering (see
//! [`schema`]); the Chromium key-coding constants are reused from
//! [`forensicnomicon_core`] via the IndexedDB reader.
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod chat;
mod contact;
mod crypto;
mod error;
mod media;
mod message;
pub mod schema;
mod timeline;
mod value;

pub use chat::Chat;
pub use chromium_storage_indexeddb::{
    decode_records, read_dir, IdbKey, IndexedDbRecord, RecordValue, V8Value,
};
pub use contact::Contact;
pub use crypto::decrypt_body;
pub use error::WaError;
pub use media::MediaMeta;
pub use message::{EncryptedBody, Message, MessageBody};
pub use timeline::{epoch_secs_to_rfc3339, TimelineEntry};
